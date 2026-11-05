use super::Bounds;
use skate3_core::{math::Vector3, player::offboard::ground_query::Frame};
pub(super) fn add(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
pub(super) fn sub(a: Vector3, b: Vector3) -> Vector3 {
    Vector3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
pub(super) fn scale(a: Vector3, s: f32) -> Vector3 {
    Vector3::new(a.x * s, a.y * s, a.z * s)
}
fn madd(a: Vector3, s: f32, b: Vector3) -> Vector3 {
    Vector3::new(
        a.x.mul_add(s, b.x),
        a.y.mul_add(s, b.y),
        a.z.mul_add(s, b.z),
    )
}
pub(super) fn point(f: Frame, p: Vector3) -> Vector3 {
    madd(
        f.forward,
        p.z,
        madd(f.up, p.y, madd(f.right, p.x, f.position)),
    )
}
pub(super) fn bounds(f: Frame, b: Bounds) -> Bounds {
    let c = point(f, scale(add(b.max, b.min), 0.5));
    let e = scale(sub(b.max, b.min), 0.5);
    let abs = |v: Vector3| Vector3::new(v.x.abs(), v.y.abs(), v.z.abs());
    let e = madd(
        abs(f.forward),
        e.z,
        madd(abs(f.up), e.y, scale(abs(f.right), e.x)),
    );
    Bounds {
        min: sub(c, e),
        max: add(c, e),
    }
}
pub(super) fn overlaps(a: Bounds, b: Bounds) -> bool {
    a.min.x <= b.max.x
        && a.max.x >= b.min.x
        && a.min.y <= b.max.y
        && a.max.y >= b.min.y
        && a.min.z <= b.max.z
        && a.max.z >= b.min.z
}
pub(super) fn matches(a: i32, b: i32) -> bool {
    a == -1 || b == -1 || a == b
}
//82ACA4E0..550: face is formed from decoded WORLD vertices and refined twice.
pub(super) fn face([a, b, c]: [Vector3; 3]) -> Vector3 {
    let u = sub(b, a);
    let v = sub(c, a);
    let n = Vector3::new(
        (-u.z).mul_add(v.y, u.y * v.z),
        (-u.x).mul_add(v.z, u.z * v.x),
        (-u.y).mul_add(v.x, u.x * v.y),
    );
    let q = n.z.mul_add(n.z, n.y.mul_add(n.y, n.x * n.x));
    let mut r = 1.0 / q.sqrt();
    for _ in 0..2 {
        r = (r * 0.5).mul_add((-q).mul_add(r * r, 1.0), r);
    }
    scale(n, r)
}
