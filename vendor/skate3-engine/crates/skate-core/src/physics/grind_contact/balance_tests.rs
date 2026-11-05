use super::*;

const X: V = [1.0, 0.0, 0.0, 0.0];
const Y: V = [0.0, 1.0, 0.0, 0.0];
const Z: V = [0.0, 0.0, 1.0, 0.0];

// Explicit synthetic geometry tests branches, not a substitute runtime map.
fn surface(kind: GeometryType) -> GrindSurface {
    GrindSurface {
        center: [0.0; 4],
        far_points: [negate(Z), Z],
        upmost_normal: Y,
        direction: Z,
        high_side: X,
        normal_limits: [0.0, 6.0],
        kind,
        audio_surface: 0,
        physics_surface: 0,
        flags: 0,
        tilted_upmost_normal: Y,
    }
}

fn contact(surface: &GrindSurface, kind: u32, entry_kind: EntryKind) -> BalanceContact<'_> {
    BalanceContact {
        surface,
        primitive_direction: Z,
        directed_grind_direction: Z,
        kind,
        entry_kind,
    }
}

fn vectors() -> BalanceVectors {
    BalanceVectors {
        grind_normal: Y,
        target_up: Y,
    }
}

fn exit_input(dt: f32) -> ExitLeanInput {
    ExitLeanInput {
        category: 100,
        current_state: 100,
        grind_substate: 0,
        timestep: dt,
    }
}

fn graph() -> PointGraph<8> {
    PointGraph {
        x: [0., 1., 2., 3., 4., 5., 6., 7.],
        y: [0., 10., 20., 30., 40., 50., 60., 70.],
    }
}

fn close(a: V, b: V) {
    for i in 0..4 {
        assert!((a[i] - b[i]).abs() < 0.00001, "lane{i}: {a:?} != {b:?}");
    }
}

#[test]
fn constructor_and_invalid_target_preserve_the_source_history() {
    let mut state = BalanceState::default();
    assert_eq!(
        state,
        BalanceState {
            elapsed: 0.,
            exit_angle_degrees: 0.,
            entry_delay: 2.,
            frames_away: 21,
            previous_normal: Y,
            exit_direction: X
        }
    );
    let original = state;
    let mut output = BalanceVectors {
        grind_normal: X,
        target_up: Z,
    };
    state.update_target_up(
        TargetUpInput {
            category: 400,
            previous_state: 402,
            board_up: Y,
        },
        None,
        &mut output,
    );
    assert_eq!(state, original);
    assert_eq!(
        output,
        BalanceVectors {
            grind_normal: X,
            target_up: Z
        }
    );
}

#[test]
fn target_selects_previous_tip_state_and_separate_tilted_normal() {
    let mut geometry = surface(GeometryType::Ledge);
    geometry.tilted_upmost_normal = X;
    let candidate = contact(&geometry, 1, EntryKind::StayInGrind);
    for previous_state in [400, 402, 404] {
        let mut state = BalanceState::default();
        let mut output = vectors();
        state.update_target_up(
            TargetUpInput {
                category: 400,
                previous_state,
                board_up: Y,
            },
            Some(&candidate),
            &mut output,
        );
        if previous_state == 400 {
            let raw = [f32::from_bits(0x3d99_9998), 0.925, 0., 0.];
            let expected = scale(raw, 1.0 / (raw[0] * raw[0] + raw[1] * raw[1]).sqrt());
            close(output.grind_normal, expected);
        } else {
            close(output.grind_normal, Y);
        }
        assert_eq!(state.previous_normal, output.grind_normal);
    }
}

#[test]
fn target_non_grind_uses_current_up_and_projection_fallback() {
    let geometry = surface(GeometryType::ThinRail);
    let candidate = contact(&geometry, 0, EntryKind::AirToGrind);
    let mut state = BalanceState::default();
    let mut output = vectors();
    state.update_target_up(
        TargetUpInput {
            category: 200,
            previous_state: 402,
            board_up: X,
        },
        Some(&candidate),
        &mut output,
    );
    close(output.grind_normal, X);
    state.update_target_up(
        TargetUpInput {
            category: 200,
            previous_state: 402,
            board_up: Z,
        },
        Some(&candidate),
        &mut output,
    );
    close(output.grind_normal, Y); // projected vector is exactly zero
}

#[test]
fn target_clamps_to_fourteen_degrees_with_the_native_rotation() {
    let geometry = surface(GeometryType::Ledge);
    let candidate = contact(&geometry, 1, EntryKind::StayInGrind);
    let mut state = BalanceState::default();
    let mut output = vectors();
    state.update_target_up(
        TargetUpInput {
            category: 400,
            previous_state: 400,
            board_up: X,
        },
        Some(&candidate),
        &mut output,
    );
    let angle = 14.0_f32.to_radians();
    close(output.target_up, [angle.sin(), angle.cos(), 0., 0.]);
    close(state.previous_normal, Y);
    // Anti-parallel target: its zero cross axis selects grind_normal.
    state.update_target_up(
        TargetUpInput {
            category: 400,
            previous_state: 400,
            board_up: negate(Y),
        },
        Some(&candidate),
        &mut output,
    );
    close(output.target_up, Y);
}

#[test]
fn quaternion_rotation_preserves_axis_component_and_direction() {
    let half_pi = core::f32::consts::FRAC_PI_2;
    close(rotate(Z, X, half_pi), Y);
    close(rotate(Z, Y, half_pi), negate(X));
    close(rotate(Y, Z, half_pi), X);
    close(rotate(Z, [1., 0., 2., 0.], half_pi), [0., 1., 2., 0.]);
    close(rotate(Z, X, -half_pi), negate(Y));
}

#[test]
fn exit_entry_delays_and_tipslide_geometry_eligibility() {
    for (entry, delay) in [
        (EntryKind::RideFromBelow, 0.8),
        (EntryKind::RideIntoCoping, 0.1),
        (EntryKind::RideFromAbove, 2.),
        (EntryKind::StayInGrind, 2.),
        (EntryKind::ChangeGrind, 2.),
        (EntryKind::AirToGrind, 2.),
        (EntryKind::DropIn, 2.),
    ] {
        let geometry = surface(GeometryType::Ledge);
        let mut state = BalanceState::default();
        state.update_exit_lean(
            exit_input(0.01),
            Some(&contact(&geometry, 1, entry)),
            &graph(),
            &mut vectors(),
        );
        assert_eq!(state.entry_delay, delay);
        assert_eq!(state.elapsed, 0.01);
        assert_eq!(state.frames_away, 0);
        assert_eq!(state.exit_angle_degrees, 0.);
    }
    for geometry_type in [
        GeometryType::ThinRail,
        GeometryType::FatRail,
        GeometryType::Ledge,
    ] {
        for kind in [0, 1, 2, 3, 4, 5] {
            let geometry = surface(geometry_type);
            let mut state = BalanceState::default();
            state.update_exit_lean(
                exit_input(0.01),
                Some(&contact(&geometry, kind, EntryKind::RideFromAbove)),
                &graph(),
                &mut vectors(),
            );
            let starts = geometry_type == GeometryType::Ledge
                || (kind == 2 && geometry_type != GeometryType::ThinRail);
            assert_eq!(state.elapsed > 0., starts, "kind{kind} {geometry_type:?}");
        }
    }
}

#[test]
fn exit_substate_two_freezes_time_and_zero_dt_does_not_update_side() {
    let mut geometry = surface(GeometryType::Ledge);
    geometry.high_side = negate(X);
    let candidate = contact(&geometry, 1, EntryKind::RideIntoCoping);
    let mut state = BalanceState::default();
    let mut input = exit_input(0.2);
    input.grind_substate = 2;
    state.update_exit_lean(input, Some(&candidate), &graph(), &mut vectors());
    assert_eq!(state.elapsed, 0.);
    assert_eq!(state.entry_delay, 0.1);
    assert_eq!(state.exit_direction, X);
    input.grind_substate = 0;
    state.update_exit_lean(input, Some(&candidate), &graph(), &mut vectors());
    assert_eq!(state.exit_direction, negate(X));
    let old_time = state.elapsed;
    input.grind_substate = 2;
    state.update_exit_lean(input, Some(&candidate), &graph(), &mut vectors());
    assert_eq!(state.elapsed, old_time);
    let mut state = BalanceState::default();
    state.update_exit_lean(exit_input(0.), Some(&candidate), &graph(), &mut vectors());
    assert_eq!(state.exit_direction, X);
}

#[test]
fn exit_history_retains_twenty_away_frames_then_resets_only_timers() {
    let geometry = surface(GeometryType::Ledge);
    let mut state = BalanceState::default();
    state.update_exit_lean(
        exit_input(0.2),
        Some(&contact(&geometry, 1, EntryKind::RideIntoCoping)),
        &graph(),
        &mut vectors(),
    );
    let mut output = BalanceVectors {
        grind_normal: X,
        target_up: Z,
    };
    for count in 1..=20 {
        state.update_exit_lean(exit_input(0.01), None, &graph(), &mut output);
        assert_eq!(state.frames_away, count);
        assert!(state.elapsed > 0.);
    }
    assert_eq!(
        output,
        BalanceVectors {
            grind_normal: X,
            target_up: Z
        }
    );
    state.previous_normal = Z;
    state.exit_direction = negate(X);
    state.update_exit_lean(exit_input(0.01), None, &graph(), &mut output);
    assert_eq!(
        (state.elapsed, state.exit_angle_degrees, state.entry_delay),
        (0., 0., 2.)
    );
    assert_eq!(state.previous_normal, Z);
    assert_eq!(state.exit_direction, negate(X));
    state.frames_away = i32::MAX;
    state.elapsed = 1.;
    state.update_exit_lean(exit_input(0.01), None, &graph(), &mut output);
    assert_eq!(state.frames_away, i32::MIN);
    assert!(state.elapsed > 1.); // native signed comparison, not unsigned/clamped
}

#[test]
fn grind_category_and_current_701_keep_existing_timer_alive_without_contact() {
    for (category, current_state) in [(400, 401), (700, 701)] {
        let mut state = BalanceState {
            elapsed: 0.5,
            frames_away: 20,
            ..BalanceState::default()
        };
        let input = ExitLeanInput {
            category,
            current_state,
            ..exit_input(0.1)
        };
        state.update_exit_lean(input, None, &graph(), &mut vectors());
        assert_eq!(state.frames_away, 0);
        assert_eq!(state.elapsed, 0.6);
    }
}

#[test]
fn exit_side_continuity_and_both_output_rotations_leave_history_unrotated() {
    let mut geometry = surface(GeometryType::Ledge);
    let mut state = BalanceState::default();
    let mut output = vectors();
    let angle = state.update_exit_lean(
        exit_input(0.6),
        Some(&contact(&geometry, 1, EntryKind::RideIntoCoping)),
        &graph(),
        &mut output,
    );
    assert!((angle - 5.).abs() < 0.00001);
    let radians = angle.to_radians();
    let expected = [-radians.sin(), radians.cos(), 0., 0.];
    close(output.grind_normal, expected);
    close(output.target_up, expected);
    assert_eq!(state.previous_normal, Y);
    geometry.high_side = negate(X);
    state.update_exit_lean(
        exit_input(0.),
        Some(&contact(&geometry, 1, EntryKind::StayInGrind)),
        &graph(),
        &mut vectors(),
    );
    close(state.exit_direction, X);
    // Thin rail chooses upmost cross primitive, not travel-directed direction.
    geometry.kind = GeometryType::ThinRail;
    let mut candidate = contact(&geometry, 0, EntryKind::StayInGrind);
    candidate.directed_grind_direction = negate(Z);
    state.update_exit_lean(exit_input(0.), Some(&candidate), &graph(), &mut vectors());
    close(state.exit_direction, X);
    state.exit_direction = Z; // exact zero alignment takes the negated side
    state.update_exit_lean(exit_input(0.), Some(&candidate), &graph(), &mut vectors());
    close(state.exit_direction, negate(X));
}

#[test]
fn force_exit_thresholds_query_geometry_and_other_bits() {
    let location = [4., 3., 2., 0.];
    for (angle, expected, queries) in [
        (15., false, 0),
        (15.001, true, 1),
        (28., true, 1),
        (28.001, true, 0),
    ] {
        let state = BalanceState {
            exit_angle_degrees: angle,
            ..BalanceState::default()
        };
        let mut flags = 0x8123_4567;
        let mut calls = 0;
        state
            .update_force_exit(location, &mut flags, |probe| {
                calls += 1;
                assert_eq!(
                    probe,
                    ForceExitProbe {
                        start: [3.9, 3., 2., 0.],
                        end: [3.9, -2., 2., 0.]
                    }
                );
                Ok::<_, ()>(None)
            })
            .unwrap();
        assert_eq!(calls, queries);
        assert_eq!(flags & 0x7fff_ffff, 0x0123_4567);
        assert_eq!(flags >> 31 != 0, expected);
    }
}

#[test]
fn force_exit_normal_comparison_is_strict_and_query_errors_preserve_flags() {
    let state = BalanceState {
        exit_angle_degrees: 20.,
        ..BalanceState::default()
    };
    for (normal_x, force) in [(-0.10001, false), (-0.1, false), (-0.09999, true)] {
        let mut flags = 0x8123_4567;
        state
            .update_force_exit([0.; 4], &mut flags, |_| {
                Ok::<_, ()>(Some(ForceExitHit {
                    normal: [normal_x, 1., 0., 0.],
                }))
            })
            .unwrap();
        assert_eq!(flags >> 31 != 0, force);
    }
    let mut flags = 0x8123_4567;
    assert_eq!(
        state.update_force_exit([0.; 4], &mut flags, |_| Err("world unavailable")),
        Err("world unavailable")
    );
    assert_eq!(flags, 0x8123_4567);
}
