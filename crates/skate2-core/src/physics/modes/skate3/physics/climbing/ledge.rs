//! Static collision probes for the first authored jump/hang/mantle set.
use crate::skate3_native::math_host::*;
use skate3_core::{math::Vector3, physics::board_world::BoardWorld};

#[derive(Clone, Copy, Debug)]
pub(super) struct Ledge {
    pub anchor: Vec3,
    pub landing: Vec3,
    pub forward: Vec3,
    pub palms: [Vec3; 2],
    pub normals: [Vec3; 2],
}
fn v(p: Vec3) -> Vector3 {
    Vector3::new(p.x, p.y, p.z)
}
fn hit(world: &BoardWorld, a: Vec3, b: Vec3, radius: f32) -> Option<(Vec3, Vec3)> {
    let h = world.query_swept_line(v(a), v(b), radius).ok()??.geometry;
    Some((
        Vec3::new(h.position.x, h.position.y, h.position.z),
        Vec3::new(h.normal.x, h.normal.y, h.normal.z),
    ))
}
#[cfg(any())]
pub(super) fn find(world: &BoardWorld, feet: Vec3, facing: Vec3) -> Option<Ledge> {
    find_range(world, feet, facing, 1.90, 0.95)
}
pub(super) fn find_air(world: &BoardWorld, feet: Vec3, facing: Vec3) -> Option<Ledge> {
    find_range(world, feet, facing, 1.25, 2.6)
}
fn find_range(
    world: &BoardWorld,
    feet: Vec3,
    facing: Vec3,
    minimum: f32,
    distance: f32,
) -> Option<Ledge> {
    let facing = Vec3::new(facing.x, 0., facing.z).try_normalize()?;
    let chest = feet + Vec3::Y * 1.15;
    let (wall, normal) = hit(world, chest, chest + facing * distance, 0.)?;
    if normal.y.abs() > 0.2 || normal.dot(facing) > -0.65 {
        return None;
    }
    let forward = -Vec3::new(normal.x, 0., normal.z).normalize();
    let right = Vec3::Y.cross(forward);
    let probe = wall + forward * 0.12;
    let (top, up) = hit(
        world,
        Vec3::new(probe.x, feet.y + 2.75, probe.z),
        Vec3::new(probe.x, feet.y + minimum, probe.z),
        0.,
    )?;
    if up.y < 0.95 {
        return None;
    }
    let anchor = Vec3::new(wall.x, top.y + 0.025, wall.z) + forward * 0.055;
    // Both hands need the same ledge, not the tip of a thin pole or a corner.
    let mut palms = [Vec3::ZERO; 2];
    let mut normals = [Vec3::Y; 2];
    for (i, side) in [0.30, -0.30].into_iter().enumerate() {
        let p = wall + forward * 0.08 + right * side;
        let (p, n) = hit(
            world,
            Vec3::new(p.x, top.y + 0.15, p.z),
            Vec3::new(p.x, top.y - 0.15, p.z),
            0.,
        )?;
        palms[i] = p + n * 0.005;
        normals[i] = n;
        if (p.y - top.y).abs() > 0.06 || n.y < 0.95 {
            return None;
        }
    }
    let ledge = Ledge {
        anchor,
        landing: Vec3::new(wall.x, top.y + 0.015, wall.z) + forward * 0.62,
        forward,
        palms,
        normals,
    };
    if !clear(world, ledge) {
        return None;
    }
    // Space for the torso outside the face, and for the head as it rises.
    let outside = anchor - forward * 0.32;
    if hit(
        world,
        outside - Vec3::Y * 1.25,
        outside + Vec3::Y * 0.55,
        0.20,
    )
    .is_some()
    {
        return None;
    }
    Some(ledge)
}
pub(super) fn clear(world: &BoardWorld, ledge: Ledge) -> bool {
    let right = Vec3::Y.cross(ledge.forward);
    for offset in [
        Vec3::ZERO,
        right * 0.25,
        -right * 0.25,
        ledge.forward * 0.25,
        -ledge.forward * 0.25,
    ] {
        let p = ledge.landing + offset;
        let Some((ground, n)) = hit(world, p + Vec3::Y * 0.15, p - Vec3::Y * 0.15, 0.) else {
            return false;
        };
        if (ground.y - ledge.landing.y).abs() > 0.06 || n.y < 0.95 {
            return false;
        }
        if hit(world, p + Vec3::Y * 0.35, p + Vec3::Y * 1.55, 0.28).is_some() {
            return false;
        }
    }
    hit(
        world,
        ledge.anchor + Vec3::Y * 0.65 - ledge.forward * 0.32,
        ledge.landing + Vec3::Y * 0.65,
        0.22,
    )
    .is_none()
}
