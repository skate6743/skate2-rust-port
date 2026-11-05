//! Original KnownAir VMX geometry; independent PC estimate seeds.
use super::runtime::Runtime;
use skate3_core::{
    air::known::{AffineTransform, KnownAirMath, RestoreVelocityGeometry},
    math::Vector3,
};
pub(super) type V = [f32; 4];
pub(super) fn dot(a: V, b: V) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
pub(super) fn sub(a: V, b: V) -> V {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn scale(a: V, s: f32) -> V {
    a.map(|v| v * s)
}
pub(super) fn cross(a: V, b: V) -> V {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
pub(super) fn normalized(v: V) -> (V, f32) {
    let s = dot(v, v);
    let mut r = s.sqrt().recip();
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-s).mul_add(r * r, 1.0), r);
    }
    let l = if s == 0.0 { 0.0 } else { s * r };
    (
        if l > f32::from_bits(0x358637bd) {
            scale(v, r)
        } else {
            [0.0; 4]
        },
        l,
    )
}
pub(super) fn rotate(m: &[V; 4], v: V) -> V {
    std::array::from_fn(|i| {
        let x = m[0][i] * v[0];
        let y = m[1][i].mul_add(v[1], x);
        m[2][i].mul_add(v[2], y)
    })
}
pub(super) fn point(m: &[V; 4], v: V) -> V {
    std::array::from_fn(|i| {
        let x = m[0][i].mul_add(v[0], m[3][i]);
        let y = m[1][i].mul_add(v[1], x);
        m[2][i].mul_add(v[2], y)
    })
}
pub(super) fn xyz(v: V) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
impl KnownAirMath for Runtime<'_> {
    fn dot3(&mut self, a: V, b: V) -> f32 {
        dot(a, b)
    }
    fn divide_vector_by_scalar(&mut self, v: V, s: f32) -> V {
        let mut r = s.recip();
        for _ in 0..2 {
            r = (-s).mul_add(r, 1.0).mul_add(r, r);
        }
        scale(v, r)
    }
    fn normalize_safe_with_length(&mut self, v: V) -> (V, f32) {
        normalized(v)
    }
    fn signed_angle_about_axis(&mut self, a: V, b: V, axis: V) -> f32 {
        if !(dot(a, a) * dot(b, b) > f32::from_bits(0x37800000)) {
            return 0.0;
        }
        let a = sub(a, scale(axis, dot(axis, a)));
        let b = sub(b, scale(axis, dot(axis, b)));
        skate3_core::riding::collision_response::signed_angle(xyz(a), xyz(b), xyz(axis))
    }
    fn wrap_angle_8258db98(&mut self, mut a: f32) -> f32 {
        let pi = f32::from_bits(0x40490fdb);
        let tau = f32::from_bits(0x40c90fdb);
        if a < -pi || !(a < pi) {
            let n = (a * f32::from_bits(0x3e22f983)) as i32;
            a = (-(n as f32)).mul_add(tau, a);
            if !(a < pi) {
                a -= tau;
            } else if a < -pi {
                a += tau;
            }
        }
        a
    }
    fn restore_velocity_geometry_82d35998(&mut self, v: V, n: V) -> RestoreVelocityGeometry {
        let tangent = sub(v, scale(n, dot(n, v)));
        let downhill = cross(n, cross(n, [0.0, 1.0, 0.0, 0.0]));
        RestoreVelocityGeometry {
            tangential_velocity: tangent,
            landing_speed_curve_input: n[1],
            landing_speed_blend_source: dot(normalized(downhill).0, normalized(tangent).0),
        }
    }
    fn transform_point_82d36880(&mut self, m: AffineTransform, p: V) -> V {
        point(&m.vectors, p)
    }
}
