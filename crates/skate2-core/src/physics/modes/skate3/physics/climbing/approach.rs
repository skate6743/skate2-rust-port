//! Airborne reach is a presentation overlay. Stock graphs, gravity and collisions
//! retain ownership until both arm targets are physically within reach.
use super::*;
pub(super) struct Approach {
    ledge: ledge::Ledge,
    pub(super) weight: f32,
}
pub(crate) fn advance(
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    controls: &PlayerControls,
) {
    let mut runtime = std::mem::take(&mut skater.climbing);
    update(&mut runtime, physics, skater, controls);
    skater.climbing = runtime;
}
fn update(
    r: &mut Runtime,
    physics: &Skate3PhysicsState,
    skater: &mut Skate3ControllerState,
    controls: &PlayerControls,
) {
    let Some(clips) = &r.clips else {
        return;
    };
    let c = &clips.reach;
    let root = matrix(skater.animated_skeleton.roots.animation_to_world);
    let globals: Vec<_> = r
        .indices
        .iter()
        .map(|&i| matrix(skater.render_pose[i]))
        .collect();
    let mut locals: Vec<_> = globals
        .iter()
        .enumerate()
        .map(|(i, &g)| {
            Transform::from_matrix(if c.parents[i] < 0 {
                g
            } else {
                globals[c.parents[i] as usize].inverse() * g
            })
        })
        .collect();
    let state = skater.player_state.current();
    let ground_speed = Vec3::from_slice(
        &skater
            .player_input
            .physical
            .reckoning
            .vector_16
            .map(f32::from_bits)[..3],
    )
    .length();
    if state == PhysicalStateId::BipedGround && (r.ground_entry.is_empty() || ground_speed < 0.3) {
        r.ground_entry = locals.clone();
        // Native walking roots sit on the support plane. Normalize the saved
        // return pose so a lifted foot in the run cycle cannot lower that root.
        let height = c.feet(&globals).y;
        for (i, local) in r.ground_entry.iter_mut().enumerate() {
            if c.parents[i] < 0 {
                local.translation.y -= height;
            }
        }
    }
    let airborne = state == PhysicalStateId::BipedAir;
    if (!airborne && state != PhysicalStateId::BipedGround) || r.cooldown > 0. {
        r.approach = None;
        return;
    }
    let feet = root.transform_point3(c.feet(&globals));
    let velocity = Vec3::from_slice(
        &skater
            .player_input
            .physical
            .reckoning
            .vector_16
            .map(f32::from_bits)[..3],
    );
    let direction = controls
        .offboard_direction
        .map(|v| Vec3::new(v[0], 0., v[2]))
        .filter(|v| v.length_squared() > 0.04)
        .unwrap_or_else(|| {
            root.z_axis.truncate()
                * if skater.player_input.processed.flags_2476 & 4 != 0 {
                    -1.
                } else {
                    1.
                }
        });
    let moving = airborne
        || ground_speed > 0.15
        || controls
            .offboard_direction
            .is_some_and(|v| Vec2::new(v[0], v[2]).length_squared() > 0.04);
    let found = moving
        .then(|| ledge::find_air(&physics.world, feet, direction))
        .flatten();
    // Follow the approach while running. Freeze the two contacts on takeoff,
    // once the player has committed to this edge.
    if !airborne {
        if let (Some(a), Some(ledge)) = (&mut r.approach, found) {
            if a.ledge.anchor.distance(ledge.anchor) < 0.5 {
                a.ledge = ledge;
            }
        }
    }
    if r.approach.is_none() {
        if let Some(ledge) = found {
            if velocity.dot(ledge.forward) >= -0.1 {
                r.approach = Some(Approach { ledge, weight: 0. });
            }
        }
    }
    let Some(a) = &mut r.approach else {
        return;
    };
    let separation = a.ledge.anchor - feet;
    let distance = Vec2::new(separation.x, separation.z).length();
    let valid = moving
        && separation.y > 0.35
        && separation.y < 2.75
        && distance < 2.7
        && separation.dot(a.ledge.forward) > 0.
        && direction.normalize_or_zero().dot(a.ledge.forward) > 0.65
        && velocity.dot(a.ledge.forward) >= -0.3
        && ledge::clear(&physics.world, a.ledge);
    let dt = physics.settings.step.simulation.time_step;
    // Continuous distance gain, starting during the run. Ground anticipation
    // is gentle; the jump smoothly builds from it into full hand contact.
    let desired = if valid {
        reach_gain(distance, airborne)
    } else {
        0.
    };
    a.weight += (desired - a.weight) * (1. - (-dt / 0.10).exp());
    if a.weight < 0.001 && !valid {
        r.approach = None;
        return;
    }
    let weight = a.weight;
    // Only shoulders and arms borrow the authored reach. The stock jump keeps
    // its hips, legs, trajectory and timing, even during a failed attempt.
    let authored = c.sample(c.duration());
    for side in ["LEFT", "RIGHT"] {
        for suffix in ["SHOULDER", "ARM", "FOREARM", "HAND"] {
            let i = c.index(&format!("{side}{suffix}"));
            locals[i].rotation = locals[i]
                .rotation
                .slerp(authored[i].rotation, weight * 0.75);
        }
    }
    let before_ik = c.globals(&locals);
    let reachable = ["LEFT", "RIGHT"].into_iter().enumerate().all(|(i, side)| {
        let shoulder =
            root.transform_point3(before_ik[c.index(&format!("{side}ARM"))].w_axis.truncate());
        let elbow = root.transform_point3(
            before_ik[c.index(&format!("{side}FOREARM"))]
                .w_axis
                .truncate(),
        );
        let hand =
            root.transform_point3(before_ik[c.index(&format!("{side}HAND"))].w_axis.truncate());
        shoulder.distance(contacts::wrist(a.ledge, i).0)
            <= shoulder.distance(elbow) + elbow.distance(hand) - 0.015
    });
    contacts::hands(c, &mut locals, root, a.ledge, weight);
    let output = c.globals(&locals);
    for (&i, &g) in r.indices.iter().zip(&output) {
        skater.render_pose[i] = native(g);
    }
    if airborne && valid && reachable && a.weight >= 0.95 && !r.ground_entry.is_empty() {
        let deck = matrix(super::super::solve::deck_frame(&physics.board));
        r.active = Some(Attached {
            phase: Phase::Catch,
            time: 0.,
            ledge: a.ledge,
            start_root: Transform::from_matrix(root),
            entry: locals,
            fallback: skater.render_pose.clone(),
            board_world: root * output[c.index("SKATEBOARD_ROOT")],
            physical_board_world: deck,
            carry_board: skater.skateboard_controller.fields.state_448 == 1,
        });
        r.approach = None;
        skater.offboard_air_selector.reset();
        skater.offboard_feet.reset();
        skater.foot_ik.state.enable_feet(false);
        crate::skate3_native::trace::info!("Climbing: caught ledge from stock jump");
    }
}

fn reach_gain(distance: f32, airborne: bool) -> f32 {
    let proximity = smooth((2.6 - distance) / 1.8);
    proximity * if airborne { 1. } else { 0.45 }
}
#[test]
fn reach_builds_continuously_before_takeoff() {
    assert_eq!(reach_gain(2.6, false), 0.);
    assert!(reach_gain(2., false) > 0.);
    assert!(reach_gain(1.5, false) > reach_gain(2., false));
    assert!(reach_gain(0.8, false) <= 0.45);
    assert_eq!(reach_gain(0.8, true), 1.);
    for i in 1..260 {
        let x = i as f32 * 0.01;
        assert!((reach_gain(x, true) - reach_gain(x + 0.01, true)).abs() < 0.009);
    }
}
