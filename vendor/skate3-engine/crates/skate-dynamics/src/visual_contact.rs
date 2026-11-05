//! Presentation-only contact correction for buffered remote skaters. This never
//! applies impulses or changes an owned body. It prevents a current-frame car
//! from being drawn through a player whose authoritative pose is still in flight.
use crate::{SolidBody, SolidCollider};
use rapier3d::{
    parry::query::{self, ShapeCastOptions},
    prelude::*,
};

pub fn resolve(
    parts: &[SolidCollider],
    previous_parts: &[SolidCollider],
    solids: &[SolidBody],
    previous_solids: &[SolidBody],
) -> Vector {
    const SKIN: f32 = 0.01;
    let mut offset = Vector::ZERO;
    if parts.is_empty() || solids.is_empty() {
        return offset;
    }
    let previous_bounds: Vec<_> = previous_parts
        .iter()
        .map(|p| p.shape.compute_aabb(&p.pose))
        .collect();
    let current_bounds: Vec<_> = parts
        .iter()
        .map(|p| p.shape.compute_aabb(&p.pose))
        .collect();
    let mut actor_min = Vector::splat(f32::INFINITY);
    let mut actor_max = Vector::splat(f32::NEG_INFINITY);
    for b in previous_bounds.iter().chain(&current_bounds) {
        actor_min = actor_min.min(b.mins);
        actor_max = actor_max.max(b.maxs);
    }
    let overlaps = |amin: Vector, amax: Vector, bmin: Vector, bmax: Vector| {
        amin.x <= bmax.x + SKIN
            && amax.x >= bmin.x - SKIN
            && amin.y <= bmax.y + SKIN
            && amax.y >= bmin.y - SKIN
            && amin.z <= bmax.z + SKIN
            && amax.z >= bmin.z - SKIN
    };
    let current_colliders: Vec<_> = solids
        .iter()
        .flat_map(|b| &b.colliders)
        .map(|c| (c, c.shape.compute_aabb(&c.pose)))
        .collect();
    // Sweep translation between rendered frames, preserving which side of a
    // fast car hit the skater even if the car crosses them in a single frame.
    if parts.len() == previous_parts.len() {
        for body in solids {
            let Some(old_body) = previous_solids.iter().find(|old| old.id == body.id) else {
                continue;
            };
            if body.colliders.len() != old_body.colliders.len() {
                continue;
            }
            // An explicit relocation must not sweep a car across the map.
            if body
                .pose
                .translation
                .distance_squared(old_body.pose.translation)
                > 400.
            {
                continue;
            }
            for (collider, old) in body.colliders.iter().zip(&old_body.colliders) {
                let car_motion = collider.pose.translation - old.pose.translation;
                let bounds_b = old.shape.compute_aabb(&old.pose);
                let end_b = collider.shape.compute_aabb(&collider.pose);
                if !overlaps(
                    actor_min + offset.min(Vector::ZERO),
                    actor_max + offset.max(Vector::ZERO),
                    bounds_b.mins.min(end_b.mins),
                    bounds_b.maxs.max(end_b.maxs),
                ) {
                    continue;
                }
                for ((part, before), bounds_a) in
                    parts.iter().zip(previous_parts).zip(&previous_bounds)
                {
                    let motion = part.pose.translation + offset - before.pose.translation;
                    let relative = motion - car_motion;
                    if relative.length_squared() < 1e-10 {
                        continue;
                    }

                    let ra = (bounds_a.maxs - bounds_a.mins).length() * 0.5;
                    let rb = (bounds_b.maxs - bounds_b.mins).length() * 0.5;
                    let ca = (bounds_a.maxs + bounds_a.mins) * 0.5;
                    let cb = (bounds_b.maxs + bounds_b.mins) * 0.5;
                    if ca.distance_squared(cb) > (ra + rb + relative.length() + SKIN).powi(2) {
                        continue;
                    }
                    let options = ShapeCastOptions {
                        max_time_of_impact: 1.,
                        target_distance: SKIN,
                        stop_at_penetration: false,
                        compute_impact_geometry_on_penetration: true,
                    };
                    let Ok(Some(hit)) = query::cast_shapes(
                        &before.pose,
                        motion,
                        &*part.shape,
                        &old.pose,
                        car_motion,
                        &*old.shape,
                        options,
                    ) else {
                        continue;
                    };
                    let normal = old.pose.rotation * hit.normal2;
                    let inward = relative.dot(normal);
                    if normal.is_finite() && inward < -1e-6 {
                        offset += normal * (-inward * (1. - hit.time_of_impact.clamp(0., 1.)));
                    }
                }
            }
        }
    }
    // Resolve existing overlap too (spawn, rotation, first visible frame).
    // Use actual solid shapes, and translate the whole rendered rig together.
    for _ in 0..8 {
        let mut deepest: Option<(f32, Vector)> = None;
        for part in parts {
            let mut pose = part.pose;
            pose.translation += offset;
            let a = part.shape.compute_aabb(&pose);
            for (collider, b) in &current_colliders {
                if (a.mins.x > b.maxs.x + SKIN || a.maxs.x < b.mins.x - SKIN)
                    || (a.mins.y > b.maxs.y + SKIN || a.maxs.y < b.mins.y - SKIN)
                    || (a.mins.z > b.maxs.z + SKIN || a.maxs.z < b.mins.z - SKIN)
                {
                    continue;
                }
                let Ok(Some(c)) =
                    query::contact(&pose, &*part.shape, &collider.pose, &*collider.shape, SKIN)
                else {
                    continue;
                };
                if c.dist < 0.
                    && c.dist.is_finite()
                    && c.normal2.is_finite()
                    && deepest.as_ref().is_none_or(|(d, _)| c.dist < *d)
                {
                    deepest = Some((c.dist, c.normal2));
                }
            }
        }
        let Some((distance, normal)) = deepest else {
            break;
        };
        offset += normal * (-distance + SKIN);
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "manual timing probe, no machine-dependent pass threshold"]
    fn visual_contact_timing_probe() {
        use std::{hint::black_box, time::Instant};
        let points: Vec<_> = [-0.25, 0.25]
            .into_iter()
            .flat_map(|x| {
                [-0.3, 0.3]
                    .into_iter()
                    .flat_map(move |y| [-0.5, 0.5].into_iter().map(move |z| Vector::new(x, y, z)))
            })
            .collect();
        let hull = SharedShape::convex_hull(&points).unwrap();
        let mut vehicle = car(0.);
        vehicle.colliders = (0..32)
            .map(|i| SolidCollider {
                shape: hull.clone(),
                pose: Pose::translation(
                    (i % 4) as f32 * 0.5 - 0.75,
                    (i / 16) as f32 * 0.6,
                    (i / 4 % 4) as f32 - 1.5,
                ),
                friction: 0.,
            })
            .collect();
        for (label, players, cars, x) in [
            ("one nearby player, one car", 1, 1, 1.1),
            ("one player being hit, one car", 1, 1, 0.98),
            ("nine distant players, one car", 9, 1, 30.),
            ("nine nearby players, nine cars", 9, 9, 1.1),
            ("nine players being hit, nine cars", 9, 9, 0.98),
        ] {
            let solids: Vec<_> = (0..cars)
                .map(|i| {
                    let mut b = vehicle.clone();
                    b.id = i + 1;
                    for c in &mut b.colliders {
                        c.pose.translation.z += i as f32 * 6.;
                    }
                    b
                })
                .collect();
            let old: Vec<_> = solids
                .iter()
                .map(|b| {
                    let mut b = b.clone();
                    b.pose.translation.x -= 0.25;
                    for c in &mut b.colliders {
                        c.pose.translation.x -= 0.25;
                    }
                    b
                })
                .collect();
            let parts: Vec<_> = (0..33)
                .map(|i| SolidCollider {
                    shape: SharedShape::capsule_y(0.08, 0.08),
                    pose: Pose::translation(x, (i % 11) as f32 * 0.15, (i / 11) as f32 * 0.2 - 0.2),
                    friction: 0.,
                })
                .collect();
            for _ in 0..10 {
                black_box(resolve(&parts, &parts, &solids, &old));
            }
            let start = Instant::now();
            for _ in 0..200 {
                for _ in 0..players {
                    black_box(resolve(black_box(&parts), &parts, &solids, &old));
                }
            }
            println!(
                "{label}: {:.3} ms/frame (synthetic 33-part players, 32 convex hulls/car)",
                start.elapsed().as_secs_f64() * 1000. / 200.
            );
        }
    }
    fn car(x: f32) -> SolidBody {
        let mut world = crate::DynamicsWorld::default();
        world
            .spawn(crate::BodyDesc {
                shape: crate::Shape::Box {
                    half_extents: [1., 1., 2.],
                },
                position: [x, 0., 0.],
                ..Default::default()
            })
            .unwrap();
        world.solid_bodies().remove(0)
    }
    fn player(x: f32) -> Vec<SolidCollider> {
        vec![SolidCollider {
            shape: SharedShape::ball(0.3),
            pose: Pose::translation(x, 0., 0.),
            friction: 0.,
        }]
    }
    #[test]
    fn fast_car_crossing_buffered_player_keeps_player_on_impact_side() {
        let before = car(-4.);
        let after = car(4.);
        let p = player(0.);
        let offset = resolve(&p, &p, &[after], &[before]);
        assert!(offset.x > 5.29 && offset.x < 5.32, "{offset:?}");
    }
    #[test]
    fn buffered_victim_stays_outside_car_until_authoritative_pose_catches_up() {
        for fps in [30., 60., 120., 144., 400.] {
            let mut previous_car = car(-5.);
            let mut previous_player = player(0.);
            for frame in 1..=(fps as usize / 3) {
                let time = frame as f32 / fps;
                let current_car = car(-5. + 100. * time);
                // Victim reacts correctly, but observers receive/render it 150ms late.
                let desired = (-5. + 100. * (time - 0.15) + 1.31).max(0.);
                let mut displayed = player(desired);
                let correction = resolve(
                    &displayed,
                    &previous_player,
                    std::slice::from_ref(&current_car),
                    std::slice::from_ref(&previous_car),
                );
                displayed[0].pose.translation += correction;
                let contact = query::contact(
                    &displayed[0].pose,
                    &*displayed[0].shape,
                    &current_car.colliders[0].pose,
                    &*current_car.colliders[0].shape,
                    0.,
                )
                .unwrap();
                assert!(
                    contact.is_none_or(|c| c.dist >= -0.001),
                    "fps={fps}, frame={frame}"
                );
                assert!(displayed[0].pose.translation.x >= 0.);
                previous_car = current_car;
                previous_player = displayed;
            }
            // An authoritative pose beyond the car no longer needs any correction.
            let free = player(previous_car.pose.translation.x + 5.);
            let correction = resolve(
                &free,
                &previous_player,
                std::slice::from_ref(&previous_car),
                std::slice::from_ref(&previous_car),
            );
            assert!(correction.length() < 0.001);
        }
    }
    #[test]
    fn overlap_is_removed_without_sweeping_history() {
        let offset = resolve(&player(0.8), &[], &[car(0.)], &[]);
        assert!(0.8 + offset.x > 1.3);
    }
    #[test]
    fn moving_away_does_not_pull_player_back_to_car() {
        let offset = resolve(&player(3.), &player(1.31), &[car(0.)], &[car(0.)]);
        assert!(offset.length() < 1e-5);
    }
    #[test]
    fn near_miss_and_car_teleport_do_not_push_player() {
        let p = player(0.);
        let mut a = car(-4.);
        let mut b = car(4.);
        for body in [&mut a, &mut b] {
            body.pose.translation.y = 5.;
            for c in &mut body.colliders {
                c.pose.translation.y = 5.;
            }
        }
        assert!(resolve(&p, &p, &[b], &[a]).length() < 1e-5);
        assert!(resolve(&p, &p, &[car(30.)], &[car(-30.)]).length() < 1e-5);
    }
}
