//! Independent branch/order checks with supplied projection measurements.
//! These do not validate the missing native three-component dot arithmetic.
use skate_core::{
    physics::manual::state::ManualState,
    riding::grounded::manual_entry::{
        ManualGroundBodies, ManualGroundInput, ManualGroundProjection, enter_ground,
        remove_velocity_into_ground,
    },
};

struct Bodies {
    velocities: [[f32; 4]; 7],
    reads: Vec<usize>,
    writes: Vec<usize>,
    alias: bool,
}
impl Bodies {
    fn new() -> Self {
        Self {
            velocities: [[2., 3., 4., f32::from_bits(0x7fc0_0123)]; 7],
            reads: vec![],
            writes: vec![],
            alias: false,
        }
    }
}
impl ManualGroundBodies for Bodies {
    fn linear_velocity(&mut self, part: usize) -> [f32; 4] {
        self.reads.push(part);
        self.velocities[if self.alias { 0 } else { part }]
    }
    fn set_linear_velocity(&mut self, part: usize, velocity: [f32; 4]) {
        self.writes.push(part);
        self.velocities[if self.alias { 0 } else { part }] = velocity;
    }
}
struct Projection {
    speed: f32,
    samples: Vec<([f32; 4], [f32; 4])>,
    fail_at: Option<usize>,
}
impl Projection {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            samples: vec![],
            fail_at: None,
        }
    }
}
impl ManualGroundProjection for Projection {
    type Error = &'static str;
    fn normal_speed(&mut self, normal: [f32; 4], velocity: [f32; 4]) -> Result<f32, Self::Error> {
        self.samples.push((normal, velocity));
        if self.fail_at == Some(self.samples.len()) {
            Err("projection unavailable")
        } else {
            Ok(self.speed)
        }
    }
}
fn input() -> ManualGroundInput {
    ManualGroundInput {
        balance: 1.,
        ground_normal: [0., 1., 0., 55.],
        flags_2468: 0,
        flags_2472: 0,
    }
}
fn state() -> ManualState {
    ManualState {
        filtered_angle_error: 2.,
        target_angle: 3.,
        measured_angle: 4.,
        angular_correction: 5.,
        elapsed: 6.,
    }
}

#[test]
fn signed_zero_bypasses_every_read_but_unordered_balance_enters() {
    for balance in [0., -0.] {
        let mut bodies = Bodies::new();
        let mut projection = Projection::new(3.);
        remove_velocity_into_ground(
            ManualGroundInput { balance, ..input() },
            &mut bodies,
            &mut projection,
        )
        .unwrap();
        assert!(bodies.reads.is_empty());
        assert!(projection.samples.is_empty());
    }
    let mut bodies = Bodies::new();
    remove_velocity_into_ground(
        ManualGroundInput {
            balance: f32::NAN,
            ..input()
        },
        &mut bodies,
        &mut Projection::new(3.),
    )
    .unwrap();
    assert_eq!(bodies.writes, [6]);
}

#[test]
fn contact_bits_and_reversal_select_exact_groups_in_native_order() {
    for (contacts, reversed, expected) in [
        (0, false, vec![6]),
        (1 << 27, false, vec![6, 0, 1, 4]),
        (1 << 26, false, vec![6, 2, 3, 5]),
        (1 << 27, true, vec![6, 2, 3, 5]),
        (1 << 26, true, vec![6, 0, 1, 4]),
        ((1 << 26) | (1 << 27), false, vec![6, 0, 1, 4, 2, 3, 5]),
        ((1 << 26) | (1 << 27), true, vec![6, 0, 1, 4, 2, 3, 5]),
    ] {
        let mut bodies = Bodies::new();
        remove_velocity_into_ground(
            ManualGroundInput {
                flags_2468: if reversed { 1 << 20 } else { 0 },
                flags_2472: contacts,
                ..input()
            },
            &mut bodies,
            &mut Projection::new(3.),
        )
        .unwrap();
        assert_eq!(bodies.reads, expected);
        assert_eq!(bodies.writes, expected);
    }
}

#[test]
fn projection_is_not_clamped_or_normalized_and_preserves_w_bits() {
    for (speed, expected_y) in [(3., -3.), (-3., 9.)] {
        let mut bodies = Bodies::new();
        let mut projection = Projection::new(speed);
        let normal = [0., 2., 0., 55.];
        remove_velocity_into_ground(
            ManualGroundInput {
                ground_normal: normal,
                ..input()
            },
            &mut bodies,
            &mut projection,
        )
        .unwrap();
        assert_eq!(projection.samples[0].0, normal);
        assert_eq!(bodies.velocities[6][..3], [2., expected_y, 4.]);
        assert_eq!(bodies.velocities[6][3].to_bits(), 0x7fc0_0123);
    }
}

#[test]
fn multiply_and_subtract_have_separate_rounding() {
    let mut bodies = Bodies::new();
    bodies.velocities[6][0] = 1.;
    let normal_x = f32::from_bits(0x3f80_0001);
    let speed = f32::from_bits(0x3f7f_fffe);
    remove_velocity_into_ground(
        ManualGroundInput {
            ground_normal: [normal_x, 0., 0., 0.],
            ..input()
        },
        &mut bodies,
        &mut Projection::new(speed),
    )
    .unwrap();
    assert_eq!(bodies.velocities[6][0].to_bits(), 0);
    assert_ne!((-normal_x).mul_add(speed, 1.).to_bits(), 0);
}

#[test]
fn later_parts_read_prior_writes_when_bodies_alias() {
    let mut bodies = Bodies::new();
    bodies.alias = true;
    let mut projection = Projection::new(1.);
    remove_velocity_into_ground(
        ManualGroundInput {
            flags_2472: 1 << 27,
            ..input()
        },
        &mut bodies,
        &mut projection,
    )
    .unwrap();
    let sampled_y: Vec<_> = projection
        .samples
        .iter()
        .map(|sample| sample.1[1])
        .collect();
    assert_eq!(sampled_y, [3., 2., 1., 0.]);
    assert_eq!(bodies.velocities[0][1], -1.);
}

#[test]
fn category100_scales_preserved_state_and_skips_velocity_work() {
    let mut state = state();
    let mut bodies = Bodies::new();
    let mut projection = Projection::new(3.);
    enter_ground(&mut state, 100, 0.5, input(), &mut bodies, &mut projection).unwrap();
    assert_eq!(
        state,
        ManualState {
            filtered_angle_error: 1.,
            angular_correction: 2.5,
            ..crate::state()
        }
    );
    assert!(bodies.reads.is_empty());
    assert!(projection.samples.is_empty());
}

#[test]
fn other_category_resets_then_runs_velocity_continuation_and_retains_error_prefix() {
    let mut state = state();
    let mut bodies = Bodies::new();
    let mut projection = Projection::new(3.);
    projection.fail_at = Some(3);
    let result = enter_ground(
        &mut state,
        200,
        77.,
        ManualGroundInput {
            flags_2472: 1 << 27,
            ..input()
        },
        &mut bodies,
        &mut projection,
    );
    assert_eq!(result, Err("projection unavailable"));
    assert_eq!(
        state,
        ManualState {
            filtered_angle_error: 0.,
            target_angle: 0.,
            measured_angle: 0.,
            angular_correction: 0.,
            elapsed: 0.,
        }
    );
    assert_eq!(bodies.reads, [6, 0, 1]);
    assert_eq!(bodies.writes, [6, 0]);
    assert_eq!(bodies.velocities[6][1], 0.);
    assert_eq!(bodies.velocities[0][1], 0.);
    assert_eq!(bodies.velocities[1][1], 3.);
}
