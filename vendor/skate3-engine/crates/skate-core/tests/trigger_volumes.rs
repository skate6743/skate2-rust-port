use skate_core::triggers::{
    Cylinder, OrientedBox, QueryShape, Tracker, Transition, TriggerEvent, distance, overlaps,
};

fn upright(x: f32, y: f32, z: f32, half_height: f32, radius: f32) -> Cylinder {
    Cylinder {
        center: [x, y, z],
        axis: [0., 1., 0.],
        half_height,
        radius,
    }
}

fn yawed(center: [f32; 3], half: [f32; 3], yaw: f32) -> OrientedBox {
    let (s, c) = yaw.sin_cos();
    OrientedBox {
        center,
        axes: [[c, 0., -s], [0., 1., 0.], [s, 0., c]],
        half_extents: half,
        fatness: 0.,
    }
}

#[test]
fn retail_query_cylinder_from_three_points() {
    // A standing skater as the recomp measured it: feet on the ground, head 1.62 m
    // and hips 0.98 m above. Half-height 0.5*1.62+0.05; 2 cm below the feet to 8 cm
    // above the head; the axis points from the hips to the feet.
    let c = QueryShape::RETAIL
        .cylinder([0., 0., 0.], [0., 1.62, 0.], [0., 0.98, 0.])
        .unwrap();
    assert_eq!(c.radius, 0.34);
    assert!((c.half_height - 0.86).abs() < 1e-6);
    assert!((c.center[1] - 0.84).abs() < 1e-6);
    assert!((c.center[1] - c.half_height - -0.02).abs() < 1e-6);
    assert!((c.center[1] + c.half_height - 1.70).abs() < 1e-6);
    assert_eq!(c.axis, [0., -1., 0.]);
    // The axis follows feet - hips, not feet - head.
    let leaning = QueryShape::RETAIL
        .cylinder([0., 0., 0.], [0., 1.62, 0.], [-1., 1., 0.])
        .unwrap();
    assert!((leaning.axis[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    assert!((leaning.axis[1] + std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    // Coincident points: straight down from the feet, minimum length.
    let point = QueryShape::RETAIL
        .cylinder([0., 0., 0.], [0., 0., 0.], [0., 0., 0.])
        .unwrap();
    assert_eq!(point.axis, [0., -1., 0.]);
    assert!((point.half_height - 0.05).abs() < 1e-7);
    assert!(
        QueryShape::RETAIL
            .cylinder([f32::NAN, 0., 0.], [0.; 3], [0.; 3])
            .is_none()
    );
}

#[test]
fn query_cylinder_matches_a_recomp_sample() {
    // The recomp's player at PCU Library (TRIGQRY 90428 ms, session trig_questions):
    // head and hips as logged, feet rebuilt on the logged axis; the builder's
    // output centre and axis must come back.
    let head = [308.6950, 75.6429, -439.0241];
    let hips = [308.6946, 74.9921, -438.9733];
    let feet = [308.6855, 74.0002, -438.9550];
    let c = QueryShape::RETAIL.cylinder(feet, head, hips).unwrap();
    for (got, want) in c.center.iter().zip([308.6933, 74.8521, -438.9707]) {
        assert!((got - want).abs() < 2e-3, "centre {:?}", c.center);
    }
    for (got, want) in c.axis.iter().zip([-0.0092, -0.9998, 0.0184]) {
        assert!((got - want).abs() < 2e-3, "axis {:?}", c.axis);
    }
}

#[test]
fn cylinder_against_axis_aligned_box_distances() {
    let unit = OrientedBox::axis_aligned([-1., -1., -1.], [1., 1., 1.]);
    assert_eq!(distance(&upright(0., 0., 0., 1., 0.34), &unit), 0.);
    assert!((distance(&upright(5., 0., 0., 1., 0.34), &unit) - 3.66).abs() < 1e-5);
    assert!((distance(&upright(0., 4., 0., 1., 0.34), &unit) - 2.).abs() < 1e-5);
    // Touching counts (tolerance 0); f32 inputs, so test just inside and just outside.
    assert!(overlaps(&upright(1.3399, 0., 0., 1., 0.34), &unit));
    assert!(!overlaps(&upright(1.341, 0., 0., 1., 0.34), &unit));
}

#[test]
fn cylinder_cap_is_flat_not_a_capsule() {
    // A box 0.2 m above the top cap: a capsule of the same radius would reach it
    // (rounded end to y 1.34), the retail cylinder (RW type 5) does not.
    let c = upright(0., 0., 0., 1., 0.34);
    assert!(!overlaps(
        &c,
        &OrientedBox::axis_aligned([0.0, 1.2, -0.1], [1.0, 2.0, 0.1])
    ));
    assert!(
        (distance(
            &c,
            &OrientedBox::axis_aligned([0.0, 1.2, -0.1], [1.0, 2.0, 0.1])
        ) - 0.2)
            .abs()
            < 1e-5
    );
    // Its rim reaches sideways at full radius right up to the cap.
    assert!(overlaps(
        &c,
        &OrientedBox::axis_aligned([0.339, 0.99, -0.1], [1.0, 2.0, 0.1])
    ));
}

#[test]
fn rotated_box_uses_its_axes_not_its_bounds() {
    // 45 degree square like SkateSchool's inthehub_vol_02 / wrongway_vol_01.
    let b = yawed([0., 0., 0.], [3., 1., 3.], std::f32::consts::FRAC_PI_4);
    let (lo, hi) = b.aabb();
    assert!((hi[0] - 3. * std::f32::consts::SQRT_2).abs() < 1e-5 && (lo[0] + hi[0]).abs() < 1e-5);
    // Inside the bounds' corner, outside the box.
    let corner = upright(3.8, 0., 3.8, 0.5, 0.1);
    assert!(!overlaps(&corner, &b));
    assert!(overlaps(&upright(3.9, 0., 0., 0.5, 0.1), &b));
    assert!(b.contains_point([4.2, 0., 0.]));
    assert!(!b.contains_point([3.5, 0., 3.5]));
}

#[test]
fn gjk_matches_analytic_upright_cylinder_box_distance() {
    // Deterministic LCG; analytic distance for an upright cylinder vs an axis-aligned box.
    let mut seed = 0x2C70_1706u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for _ in 0..2000 {
        let lo = [next() * 20. - 10., next() * 20. - 10., next() * 20. - 10.];
        let hi = [
            lo[0] + next() * 6. + 0.01,
            lo[1] + next() * 6. + 0.01,
            lo[2] + next() * 6. + 0.01,
        ];
        let b = OrientedBox::axis_aligned(lo, hi);
        let c = upright(
            next() * 30. - 15.,
            next() * 30. - 15.,
            next() * 30. - 15.,
            next() * 2. + 0.01,
            next() + 0.05,
        );
        let gy =
            ((c.center[1] - b.center[1]).abs() - c.half_height - b.half_extents[1]).max(0.) as f64;
        let dx = ((c.center[0] - b.center[0]).abs() - b.half_extents[0]).max(0.) as f64;
        let dz = ((c.center[2] - b.center[2]).abs() - b.half_extents[2]).max(0.) as f64;
        let gxz = ((dx * dx + dz * dz).sqrt() - c.radius as f64).max(0.);
        let expected = (gy * gy + gxz * gxz).sqrt();
        let got = distance(&c, &b);
        assert!(
            (got - expected).abs() < 2e-4 * (1. + expected),
            "{c:?} {b:?}: {got} vs {expected}"
        );
    }
}

#[test]
fn gjk_rotated_cases_agree_with_point_sampling() {
    // Overlap iff some sampled cylinder point lies in the box (conservative check
    // only where the sampled answer is unambiguous).
    let mut seed = 7u32;
    let mut next = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for _ in 0..300 {
        let b = yawed(
            [next() * 4. - 2., next() * 2. - 1., next() * 4. - 2.],
            [next() * 2. + 0.2, next() + 0.2, next() * 2. + 0.2],
            next() * 6.28,
        );
        let c = upright(
            next() * 8. - 4.,
            next() * 4. - 2.,
            next() * 8. - 4.,
            next() + 0.1,
            next() * 0.5 + 0.05,
        );
        let d = distance(&c, &b);
        let mut hit = false;
        for i in 0..=8 {
            for j in 0..16 {
                for k in 0..=4 {
                    let a = j as f32 / 16. * std::f32::consts::TAU;
                    let r = c.radius * k as f32 / 4.;
                    let p = [
                        c.center[0] + r * a.cos(),
                        c.center[1] - c.half_height + 2. * c.half_height * i as f32 / 8.,
                        c.center[2] + r * a.sin(),
                    ];
                    hit |= b.contains_point(p);
                }
            }
        }
        if hit {
            assert_eq!(d, 0., "{c:?} {b:?}");
        }
        if d > 0.05 {
            assert!(!hit);
        }
    }
}

#[test]
fn tracker_posts_enters_then_exits_per_body_in_order() {
    let a = OrientedBox::axis_aligned([0., 0., 0.], [2., 2., 2.]);
    let b = OrientedBox::axis_aligned([10., 0., 0.], [12., 2., 2.]);
    let volumes = [("a", a), ("b", b)];
    let mut tracker = Tracker::<&str, &str>::default();
    let inside_a = upright(1., 1., 1., 0.5, 0.2);
    let inside_b = upright(11., 1., 1., 0.5, 0.2);
    let ev = tracker.update(&[("player", inside_a)], &volumes);
    assert_eq!(
        ev,
        vec![TriggerEvent {
            body: "player",
            volume: "a",
            transition: Transition::Entered
        }]
    );
    assert!(tracker.update(&[("player", inside_a)], &volumes).is_empty());
    let ev = tracker.update(&[("player", inside_b), ("ped", inside_a)], &volumes);
    assert_eq!(
        ev.iter()
            .map(|e| (e.body, e.volume, e.transition))
            .collect::<Vec<_>>(),
        vec![
            ("player", "b", Transition::Entered),
            ("player", "a", Transition::Exited),
            ("ped", "a", Transition::Entered),
        ]
    );
    assert_eq!(tracker.inside(&"player"), &["b"]);
}

#[test]
fn removing_a_volume_is_silent_removing_a_body_exits() {
    let a = OrientedBox::axis_aligned([0., 0., 0.], [2., 2., 2.]);
    let b = OrientedBox::axis_aligned([0., 0., 0.], [3., 3., 3.]);
    let mut tracker = Tracker::<u32, u32>::default();
    let inside = upright(1., 1., 1., 0.5, 0.2);
    assert_eq!(tracker.update(&[(1, inside)], &[(10, a), (20, b)]).len(), 2);
    // Volume 10 unloads while the body is inside it: no exit (RemoveVolume 82DD7018).
    assert!(tracker.update(&[(1, inside)], &[(20, b)]).is_empty());
    // Coming back is a fresh enter.
    assert_eq!(
        tracker.update(&[(1, inside)], &[(10, a), (20, b)]),
        vec![TriggerEvent {
            body: 1,
            volume: 10,
            transition: Transition::Entered
        }]
    );
    // The body stops being tracked: exits for every volume it was in (82DD6D20).
    let ev = tracker.update(&[], &[(10, a), (20, b)]);
    assert_eq!(
        ev.iter()
            .map(|e| (e.volume, e.transition))
            .collect::<Vec<_>>(),
        vec![(10, Transition::Exited), (20, Transition::Exited)]
    );
    assert!(tracker.bodies().next().is_none());
}

#[test]
fn box_validity() {
    assert!(OrientedBox::axis_aligned([0.; 3], [1.; 3]).is_valid());
    let mut bad = OrientedBox::axis_aligned([0.; 3], [1.; 3]);
    bad.axes[0] = [2., 0., 0.];
    assert!(!bad.is_valid());
    bad = OrientedBox::axis_aligned([0.; 3], [1.; 3]);
    bad.half_extents[1] = f32::NAN;
    assert!(!bad.is_valid());
}
