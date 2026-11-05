//! Independent caller/branch checks; no generated executable fixtures.
use skate_core::{
    math::Vector3,
    physics::{
        force_queue::{BoardForceQueue, QueuedPointForce},
        manual::controller::ManualEffect,
        rigid_body::RetailInertiaDynamics,
    },
    riding::{
        braking::{BrakeSettings, LinearDragSettings},
        grounded::{
            drag::{
                BodyInertias, DRAG_FREQUENCY, DragBindingError, DragSelection, GroundDragInput,
            },
            propulsion::{
                self, GroundPropulsionInput, GroundPropulsionSettings, PropulsionSubmission,
            },
        },
    },
};

fn input() -> GroundPropulsionInput {
    GroundPropulsionInput {
        flags_2468: (1 << 30) | (1 << 25),
        flags_2472: 0,
        target_speed: 10.,
        signed_speed: 4.,
        absolute_body_speed: 4.,
        scalar_2660: 2.,
        timestep: 0.25,
        brake_input: 0.5,
        push_direction: Vector3::new(0., 2., 3.),
        brake_direction: Vector3::new(1., 0., 0.),
        surface_braking_factor: 0.2,
    }
}
fn settings() -> GroundPropulsionSettings {
    GroundPropulsionSettings {
        braking: BrakeSettings {
            input_force: 10.,
            override_force: 20.,
            minimum_speed: 0.5,
        },
        maximum_pushable_speed: 8.,
        mode_speed_changes: [2., 1.],
    }
}
fn manual() -> ManualEffect {
    ManualEffect {
        angular_displacement: [0.; 4],
        force_world: [0.; 4],
        force_point_body: [0.; 4],
        corrective_force_world: [11., 12., 13., 14.],
        corrective_point_body: [21., 22., 23., 24.],
        correction_active: false,
        opposing_motion_without_correction: false,
    }
}
fn inertia() -> RetailInertiaDynamics {
    RetailInertiaDynamics {
        inverse_tensor: Vector3::new(2., 3., 4.),
        inverse_mass: 5.,
        spherical: 6.,
        maximum_linear_velocity: 7.,
        maximum_angular_velocity: 8.,
        linear_drag: 9.,
        angular_drag: 10.,
    }
}

#[test]
fn live_processed_scale_time_and_distinct_axes_reach_brake_then_push_records() {
    let mut suppressed = 255;
    let forces = propulsion::calculate(input(), settings(), &mut suppressed);
    assert_eq!(suppressed, 0);
    assert_eq!(forces.braking.force_world, Vector3::new(-1., -0., -0.));
    assert_eq!(forces.push.vector, Vector3::new(0., 24., 36.));
    let mut queue = BoardForceQueue::default();
    assert_eq!(
        forces.submit(&manual(), &mut queue),
        PropulsionSubmission::BrakingAndPush([true, true])
    );
    assert_eq!(
        queue.entries().iter().map(|f| f.tag).collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(queue.entries()[0].point_body, Vector3::ZERO);
    assert_eq!(queue.entries()[1].point_body, Vector3::ZERO);
    let doubled = propulsion::calculate(
        GroundPropulsionInput {
            scalar_2660: 4.,
            ..input()
        },
        settings(),
        &mut suppressed,
    );
    assert_eq!(doubled.push.vector, Vector3::new(0., 48., 72.));
}

#[test]
fn suppression_byte_is_republished_and_zero_forces_are_still_queued() {
    let mut suppressed = 0;
    let forces = propulsion::calculate(
        GroundPropulsionInput {
            flags_2472: 1 << 9,
            ..input()
        },
        settings(),
        &mut suppressed,
    );
    assert_eq!(suppressed, 1);
    assert_eq!(forces.push.vector, Vector3::ZERO);
    let inactive = propulsion::calculate(
        GroundPropulsionInput {
            flags_2468: 0,
            flags_2472: 1 << 9,
            ..input()
        },
        settings(),
        &mut suppressed,
    );
    assert_eq!(suppressed, 0);
    let mut queue = BoardForceQueue::default();
    inactive.submit(&manual(), &mut queue);
    assert_eq!(queue.entries().len(), 2);
    assert_eq!(queue.entries()[0].force_world, Vector3::ZERO);
    assert_eq!(queue.entries()[1].force_world, Vector3::ZERO);
}

#[test]
fn only_manual_output80_selects_alternate_tag7_payload() {
    let forces = propulsion::calculate(input(), settings(), &mut 0);
    let mut queue = BoardForceQueue::default();
    let manual = ManualEffect {
        correction_active: true,
        ..manual()
    };
    assert_eq!(
        forces.submit(&manual, &mut queue),
        PropulsionSubmission::ManualCorrection(true)
    );
    assert_eq!(
        queue.entries(),
        [QueuedPointForce {
            tag: 7,
            force_world: Vector3::new(11., 12., 13.),
            point_body: Vector3::new(21., 22., 23.),
        }]
    );
    queue.clear();
    let only81 = ManualEffect {
        correction_active: false,
        opposing_motion_without_correction: true,
        ..manual
    };
    assert_eq!(
        forces.submit(&only81, &mut queue),
        PropulsionSubmission::BrakingAndPush([true, true])
    );
}

#[test]
fn capacity_drops_later_push_without_reordering_or_clearing_prior_forces() {
    let mut queue = BoardForceQueue::default();
    for tag in 100..120 {
        queue.append(QueuedPointForce {
            tag,
            ..QueuedPointForce::default()
        });
    }
    let forces = propulsion::calculate(input(), settings(), &mut 0);
    assert_eq!(
        forces.submit(&manual(), &mut queue),
        PropulsionSubmission::BrakingAndPush([true, false])
    );
    assert_eq!(queue.entries().len(), 21);
    assert_eq!(queue.entries()[0].tag, 100);
    assert_eq!(queue.entries()[20].tag, 2);
}

#[test]
fn braking_override_reverse_and_unordered_cutoff_follow_direct_branches() {
    let mut sample = input();
    sample.flags_2468 |= 1 << 29;
    sample.signed_speed = -4.;
    let positive = propulsion::calculate(sample, settings(), &mut 0);
    assert_eq!(positive.braking.force_world.x, 4.);
    sample.absolute_body_speed = 0.25;
    assert_eq!(
        propulsion::calculate(sample, settings(), &mut 0)
            .braking
            .force_world,
        Vector3::ZERO
    );
    sample.absolute_body_speed = f32::NAN;
    assert_eq!(
        propulsion::calculate(sample, settings(), &mut 0)
            .braking
            .force_world
            .x,
        4.
    );
    sample.signed_speed = f32::NAN;
    assert_eq!(
        propulsion::calculate(sample, settings(), &mut 0)
            .braking
            .force_world
            .x,
        -4.
    );
}

#[test]
fn push_unordered_ratio_selects_upper_fraction_before_both_limit_tests() {
    // Negative limits are not sanitized by the source. They make the two
    // independent pre-clamp comparisons visible even when the input gap is0.
    let sample = GroundPropulsionInput {
        signed_speed: f32::NAN,
        absolute_body_speed: 0.,
        ..input()
    };
    let settings = GroundPropulsionSettings {
        mode_speed_changes: [2., -2.],
        ..settings()
    };
    let forces = propulsion::calculate(sample, settings, &mut 0);
    assert_eq!(forces.push.vector, Vector3::new(0., 32., 48.));
}

fn drag_input() -> GroundDragInput {
    GroundDragInput {
        flags_2468: 1 << 30,
        absolute_body_speed_2616: 0.5,
        balance_2720: 0.,
        scalar_2724: 0.,
        ground_normal_y: 0.75,
    }
}
fn drag_settings() -> LinearDragSettings {
    LinearDragSettings {
        brake_speed: 1.,
        balance_speed: 2.,
        comparison_threshold: 0.5,
        balance_drag: 0.25,
    }
}

#[test]
fn drag_reads_normal_y_and_preserves_strict_thresholds_and_exclusions() {
    let settings = drag_settings();
    assert_eq!(drag_input().calculate(settings), 1.);
    assert_eq!(
        GroundDragInput {
            scalar_2724: 0.1,
            ..drag_input()
        }
        .calculate(settings),
        0.
    );
    assert_eq!(
        GroundDragInput {
            absolute_body_speed_2616: 2.,
            ..drag_input()
        }
        .calculate(settings),
        0.
    );
    assert_eq!(
        GroundDragInput {
            balance_2720: 1.,
            ..drag_input()
        }
        .calculate(settings),
        0.25
    );
    assert_eq!(
        GroundDragInput {
            balance_2720: 1.,
            ground_normal_y: 0.5,
            ..drag_input()
        }
        .calculate(settings),
        0.
    );
    assert_eq!(
        GroundDragInput {
            balance_2720: 1.,
            ground_normal_y: f32::NAN,
            ..drag_input()
        }
        .calculate(settings),
        0.
    );
    assert_eq!(
        GroundDragInput {
            absolute_body_speed_2616: f32::NAN,
            ..drag_input()
        }
        .calculate(settings),
        0.
    );
}

#[test]
fn drag_publication_changes_only_linear_drag_of_bound_inertias() {
    let mut storage = [inertia(); 8];
    let bindings = [6, 1, 1, 5, 3, 2, 0];
    let drag = GroundDragInput {
        balance_2720: 1.,
        ..drag_input()
    }
    .calculate(drag_settings());
    BodyInertias {
        part_inertia_indices: &bindings,
        inertias: &mut storage,
    }
    .apply_ground_drag(drag)
    .unwrap();
    for index in [0, 1, 2, 3, 5, 6] {
        assert_eq!(
            storage[index],
            RetailInertiaDynamics {
                linear_drag: f32::from_bits(0x416f_ffff),
                ..inertia()
            }
        );
    }
    assert_eq!(storage[4], inertia());
    assert_eq!(storage[7], inertia());
    assert_eq!(DRAG_FREQUENCY.to_bits(), 0x426f_ffff);
}

#[test]
fn drag_all_part_count_boundaries_single_selection_and_signed_zero() {
    for count in [0, 1, 3, 4, 7, 8] {
        let mut storage = [inertia(); 8];
        let indices: Vec<_> = (0..count).collect();
        BodyInertias {
            part_inertia_indices: &indices,
            inertias: &mut storage,
        }
        .set_linear_drag(-0., DragSelection::AllParts)
        .unwrap();
        for (index, value) in storage.iter().enumerate() {
            assert_eq!(
                value.linear_drag.to_bits(),
                if index < count {
                    0x8000_0000
                } else {
                    9f32.to_bits()
                }
            );
        }
    }
    let mut storage = [inertia(); 2];
    BodyInertias {
        part_inertia_indices: &[1, 0],
        inertias: &mut storage,
    }
    .set_linear_drag(1., DragSelection::Part(0))
    .unwrap();
    assert_eq!(storage[0], inertia());
    assert_eq!(storage[1].linear_drag.to_bits(), 0x426f_ffff);
}

#[test]
fn invalid_host_bindings_fail_before_mutation() {
    let mut storage = [inertia(); 2];
    let mut bodies = BodyInertias {
        part_inertia_indices: &[0, 3],
        inertias: &mut storage,
    };
    assert_eq!(
        bodies.apply_ground_drag(1.),
        Err(DragBindingError::InertiaOutsideStorage)
    );
    assert_eq!(
        bodies.set_linear_drag(1., DragSelection::Part(usize::MAX)),
        Err(DragBindingError::PartOutsideAssembly)
    );
    assert_eq!(storage, [inertia(); 2]);
}
