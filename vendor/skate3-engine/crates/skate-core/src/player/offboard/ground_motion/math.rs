//! Original VMX operation trees expressed with ordinary PC floating-point math.
//! PC seeds and dot sums are not a bit-exact Xenon emulation claim.
use super::{Frame, Vector};
use crate::trigonometry;
pub(super) const ZERO: Vector = [0.0; 4];
pub(super) const UP: Vector = [0.0, 1.0, 0.0, 0.0];
pub(super) const IDENTITY: Frame = [[1.0, 0.0, 0.0, 0.0], UP, [0.0, 0.0, 1.0, 0.0], ZERO];
pub(super) const DT: f32 = f32::from_bits(0x3c88_8889);
pub(super) const RATE: f32 = f32::from_bits(0x426f_ffff);
pub(super) const EPSILON: f32 = f32::from_bits(0x3a83_126f);
const NORMAL_EPSILON: f32 = f32::from_bits(0x3586_37bd);
pub(super) fn add(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] + b[i])
}
pub(super) fn sub(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
pub(super) fn scale(a: Vector, b: f32) -> Vector {
    a.map(|v| v * b)
}
pub(super) fn madd(a: Vector, b: f32, c: Vector) -> Vector {
    std::array::from_fn(|i| a[i].mul_add(b, c[i]))
}
pub(super) fn dot(a: Vector, b: Vector) -> f32 {
    (a[0] * b[0] + a[1] * b[1]) + a[2] * b[2]
}
pub(super) fn cross(a: Vector, b: Vector) -> Vector {
    [
        (-a[2]).mul_add(b[1], a[1] * b[2]),
        (-a[0]).mul_add(b[2], a[2] * b[0]),
        (-a[1]).mul_add(b[0], a[0] * b[1]),
        (-a[3]).mul_add(b[3], a[3] * b[3]),
    ]
}
fn inverse_sqrt(q: f32) -> f32 {
    let mut r = 1.0 / q.sqrt();
    for _ in 0..2 {
        let square = r * r;
        let half = r * 0.5;
        r = half.mul_add((-q).mul_add(square, 1.0), r);
    }
    r
}
pub(super) fn reciprocal(d: f32) -> f32 {
    let mut r = 1.0 / d;
    for _ in 0..2 {
        r = r.mul_add((-d).mul_add(r, 1.0), r);
    }
    r
}
pub(super) fn length(v: Vector) -> f32 {
    let q = dot(v, v);
    let x = q * inverse_sqrt(q);
    if q == 0.0 { 0.0 } else { x }
}
pub(super) fn normalize(v: Vector) -> Vector {
    scale(v, inverse_sqrt(dot(v, v)))
}
pub(super) fn normalize_safe(v: Vector, fallback: Vector) -> Vector {
    let q = dot(v, v);
    let r = inverse_sqrt(q);
    let n = scale(v, r);
    let len = if q == 0.0 { 0.0 } else { q * r };
    if len > NORMAL_EPSILON { n } else { fallback }
}
pub(super) fn select(test: f32, positive: f32, negative: f32) -> f32 {
    if test >= 0.0 { positive } else { negative }
}
pub(super) fn clamp_signed(v: f32) -> f32 {
    let low = select(-1.0 - v, -1.0, v);
    select(1.0 - low, low, 1.0)
}
pub(super) fn transform_vector(v: Vector, f: Frame) -> Vector {
    madd(f[2], v[2], madd(f[1], v[1], scale(f[0], v[0])))
}
pub(super) fn transform_point(v: Vector, f: Frame) -> Vector {
    madd(f[2], v[2], madd(f[1], v[1], madd(f[0], v[0], f[3])))
}
pub(super) fn compose(a: Frame, b: Frame) -> Frame {
    [
        transform_vector(a[0], b),
        transform_vector(a[1], b),
        transform_vector(a[2], b),
        transform_point(a[3], b),
    ]
}
pub(super) fn inverse_rigid(f: Frame) -> Frame {
    crate::physics::skeleton_root::inverse_rigid(&f)
}

///82D7FEE0..FFCC XYZ Rodrigues rotation, using the host's geometric W=0
///representation rather than native permutation scratch lanes.
pub(super) fn rotate(v: Vector, axis: Vector, angle: f32) -> Vector {
    let [x, y, z, _] = axis;
    let (s, c) = trigonometry::sin_cos(angle);
    let t = 1.0 - c;
    let (tx, ty, tz) = (t * x, t * y, t * z);
    let (sx, sy, sz) = (s * x, s * y, s * z);
    let xx = tx.mul_add(x, c);
    let yx = ty * x - sz;
    let zx = tz.mul_add(x, sy);
    let columns = [
        [xx, tx.mul_add(y, sz), tx * z - sy, 0.],
        [yx, ty.mul_add(y, c), ty.mul_add(z, sx), 0.],
        [zx, tz * y - sx, tz.mul_add(z, c), 0.],
    ];
    transform_vector(v, [columns[0], columns[1], columns[2], ZERO])
}

/// Original82BD3E78, independently checked against its full raw body.
pub(super) fn limit_angle(target: Vector, from: Vector, maximum: f32) -> Vector {
    let a = normalize_safe(target, ZERO);
    let b = normalize_safe(from, ZERO);
    let axis = cross(a, b);
    let cosine = dot(a, b).max(-1.0).min(1.0);
    let angle = trigonometry::acos(cosine);
    let tau = f32::from_bits(0x40c9_0fdb);
    let turns = angle.mul_add(f32::from_bits(0x3e22_f983), 0.5).floor();
    let closest = (-turns).mul_add(tau, angle);
    if closest.abs() < maximum || dot(axis, axis) < f32::from_bits(0x3780_0000) {
        return target;
    }
    let axis = normalize(axis);
    let (s, c) = trigonometry::sin_cos(-maximum * 0.5);
    let mut q = scale(axis, s);
    q[3] = c; //82BD4130 vrlimi inserts cosine in the quaternion W lane.
    let first = cross(q, from);
    let intermediate = madd(from, c, first);
    let second = cross(q, intermediate);
    scale(madd(second, 2.0, from), length(target))
}
