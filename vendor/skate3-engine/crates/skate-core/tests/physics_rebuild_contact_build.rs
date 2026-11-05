//! Independently specified geometry and branch cases. Supplied response scales
//! are algebraic test inputs, never native hardware estimates or parity fixtures.
use skate_core::math::Vector3;
use skate_core::physics::solver::contact_build::{
    ContactBuildError, ContactMassResponse, UnavailableContactMassResponse, build_with_response,
    prepare_contact,
};

fn row(words: &mut [u32; 64], row: usize, values: [f32; 4]) {
    words[row * 4..row * 4 + 4].copy_from_slice(&values.map(f32::to_bits));
}

fn read(words: &[u32; 64], row: usize) -> [f32; 4] {
    core::array::from_fn(|i| f32::from_bits(words[row * 4 + i]))
}

fn origin() -> [u32; 64] {
    let mut words = [0; 64];
    row(&mut words, 2, [1.0, 0.0, 0.0, 0.5]);
    row(&mut words, 3, [0.0, 1.0, 0.0, 0.75]);
    row(&mut words, 4, [0.0, 0.0, 1.0, 0.25]);
    for body in 0..2 {
        row(&mut words, 8 + body, [1.0, 0.0, 0.0, 1.0]);
        row(&mut words, 10 + body, [1.0, 1.0, 0.0, 0.0]);
        words[(10 + body) * 4 + 3] = 4;
    }
    words
}

struct ExpectedResponse {
    masses: [f32; 3],
    scales: [f32; 3],
    calls: usize,
}
impl ContactMassResponse for ExpectedResponse {
    fn inverse_effective_mass(&mut self, masses: [f32; 3]) -> Result<[f32; 3], ContactBuildError> {
        assert_eq!(
            masses, self.masses,
            "builder must pass its actual effective masses"
        );
        self.calls += 1;
        Ok(self.scales)
    }
}

fn compile(words: &mut [u32; 64], dt: f32, masses: [f32; 3], scales: [f32; 3]) {
    let mut response = ExpectedResponse {
        masses,
        scales,
        calls: 0,
    };
    build_with_response(words, dt, &mut response).unwrap();
    assert_eq!(response.calls, 1);
}

#[test]
fn origin_masses_and_complete_metadata_publication() {
    let mut words = origin();
    words[3] = 0xaabbccdd;
    words[7] = 0x11223344;
    words[27] = 0x55667788;
    words[31] = 0x99aabbcc;
    words[23] = 0x12345678;
    words[43] = 4 | 8 | 0x80;
    let prepared = prepare_contact(&words, 0.25);
    assert_eq!(prepared.effective_mass, [2.0; 3]);
    assert_eq!(prepared.arms, [Vector3::ZERO; 2]);
    compile(&mut words, 0.25, [2.0; 3], [0.5; 3]);
    let mut expected = [0; 64];
    expected[3] = 0x55667788;
    expected[7] = 0x99aabbcc;
    row(&mut expected, 2, [0.5, 0.0, 0.0, 0.0]);
    expected[11] = 8;
    row(&mut expected, 3, [0.0, 0.5, 0.0, 0.75]);
    row(&mut expected, 4, [0.0, 0.0, 0.5, 0.25]);
    row(&mut expected, 7, [1.0, 0.0, 0.0, 0.0]);
    expected[31] = 0xaabbccdd;
    row(&mut expected, 8, [0.0, 0.0, 0.0, 1.0]);
    row(&mut expected, 9, [0.0, 0.0, 0.0, 1.0]);
    row(&mut expected, 10, [0.0, 1.0, 0.0, 0.0]);
    expected[43] = 0x11223344;
    row(&mut expected, 13, [0.0, 0.0, 1.0, 0.0]);
    expected[55] = 0x12345678;
    assert_eq!(words, expected);
}

#[test]
fn off_center_inertia_and_tangent_carry_are_retained() {
    let mut words = origin();
    row(&mut words, 0, [0.0, 2.0, 0.0, 0.0]);
    row(&mut words, 1, [0.0, 0.0, 3.0, 0.0]);
    // A normal torque is -2Z; B normal torque is +3Y.
    let p = prepare_contact(&words, 0.0);
    assert_eq!(p.effective_mass, [15.0, 11.0, 6.0]);
    assert_eq!(p.angular_response_a[0], [0.0, 0.0, -2.0, -2.0]);
    assert_eq!(p.angular_response_b[0], [0.0, 3.0, 0.0, 3.0]);
    compile(&mut words, 0.0, [15.0, 11.0, 6.0], [0.125, 0.25, 0.5]);
    assert_eq!(read(&words, 8), [0.0, 0.0, -2.0, 1.0]);
    assert_eq!(read(&words, 9), [0.0, 3.0, 0.0, 1.0]);
    assert_eq!(read(&words, 12), [-3.0, 0.0, 0.0, -3.0]);
    assert_eq!(read(&words, 14), [2.0, 0.0, 0.0, 2.0]);
}

#[test]
fn full_symmetric_inertia_changes_response_and_effective_mass() {
    let mut words = origin();
    row(&mut words, 0, [0.0, 2.0, 3.0, 0.0]);
    row(&mut words, 8, [4.0, 1.0, 2.0, 2.0]);
    row(&mut words, 10, [6.0, 5.0, 3.0, 0.0]);
    words[43] = 4;
    let p = prepare_contact(&words, 0.0);
    // I * (0,3,-2) = (-1,9,-3), torque work 33 + masses 3.
    assert_eq!(p.angular_response_a[0], [-1.0, 9.0, -3.0, 2.0]);
    assert_eq!(p.effective_mass, [36.0, 39.0, 19.0]);
    compile(&mut words, 0.0, [36.0, 39.0, 19.0], [0.125; 3]);
}

#[test]
fn each_active_state_gates_mass_inertia_and_point_acceleration() {
    for (a, b, masses) in [
        (false, false, [0.0; 3]),
        (true, false, [5.0, 1.0, 5.0]),
        (false, true, [10.0, 10.0, 1.0]),
        (true, true, [15.0, 11.0, 6.0]),
    ] {
        let mut words = origin();
        row(&mut words, 0, [0.0, 2.0, 0.0, 0.0]);
        row(&mut words, 1, [0.0, 0.0, 3.0, 0.0]);
        row(&mut words, 12, [1.0, 2.0, 3.0, 0.0]);
        row(&mut words, 13, [4.0, 5.0, 6.0, 0.0]);
        words[43] = if a { 4 } else { 8 };
        words[47] = if b { 4 } else { 0 };
        let p = prepare_contact(&words, 0.5);
        assert_eq!(p.active, [a, b]);
        assert_eq!(p.effective_mass, masses);
        assert_eq!(
            p.point_acceleration[0],
            if a {
                Vector3::new(1.0, 2.0, 3.0)
            } else {
                Vector3::ZERO
            }
        );
        assert_eq!(
            p.point_acceleration[1],
            if b {
                Vector3::new(4.0, 5.0, 6.0)
            } else {
                Vector3::ZERO
            }
        );
        compile(&mut words, 0.5, masses, [0.0; 3]);
        assert_eq!(words[11], if a { 0 } else { 8 });
    }
}

#[test]
fn force_torque_velocity_and_squared_timestep_predict_contact_separation() {
    let mut words = origin();
    row(&mut words, 0, [1.0, 2.0, 3.0, 0.0]);
    row(&mut words, 1, [4.0, 6.0, 8.0, 0.0]);
    row(&mut words, 7, [4.0, 6.0, 8.0, 0.0]);
    row(&mut words, 12, [10.0, 20.0, 30.0, 0.0]);
    row(&mut words, 14, [2.0, 3.0, 5.0, 0.0]);
    row(&mut words, 13, [1.0, 2.0, 3.0, 0.0]);
    row(&mut words, 5, [2.0, 4.0, 6.0, 0.0]);
    let p = prepare_contact(&words, 0.5);
    assert_eq!(
        p.point_acceleration,
        // (2,3,5) cross (1,2,3) = (-1,-1,+1).
        [Vector3::new(9.0, 19.0, 31.0), Vector3::new(1.0, 2.0, 3.0)]
    );
    assert_eq!(p.separation_projection, [3.0, 4.0, 5.0]);
    assert_eq!(p.restitution_projection, [-0.5, -1.0, -1.5]);
    assert_eq!(p.predicted_separation_projection, [2.0, 1.75, 1.0]);
    compile(&mut words, 0.5, [15.0, 12.0, 7.0], [0.5; 3]);
    assert_eq!(read(&words, 6), [1.25, 0.875, 0.5, 1.5]);
}

#[test]
fn normal_target_ordered_sign_branches_and_zero_boundaries() {
    // (separation, velocity, forceB, restitution, expected normal target).
    for (sep, vel, force, bounce, expected) in [
        (-2.0, -2.0, 8.0, 0.5_f32, 4.0), // negative separation, positive restitution
        (-2.0, 4.0, 0.0, 0.5, 6.0),      // both negative: subtract their sum
        (2.0, 2.0, 0.0, 0.5, 5.0),       // positive separation: restitution only
        (2.0, -1.0, 0.0, 0.5, 0.5),
        (2.0, 2.0, -5.0, 0.5, -1.0), // nonpositive prediction clears adjustment
        (2.0, 2.0, -4.0, 0.5, 0.0),
        (0.0, 2.0, 0.0, 0.5, 3.0),  // equality takes >= separation branch
        (-2.0, 3.0, 0.0, 0.0, 1.0), // zero restitution takes >= branch
    ] {
        let mut words = origin();
        row(&mut words, 1, [sep, 0.0, 0.0, 0.0]);
        row(&mut words, 7, [sep, 0.0, 0.0, 0.0]);
        row(&mut words, 5, [vel, 2.0, -3.0, 0.0]);
        row(&mut words, 13, [force, 0.0, 0.0, 0.0]);
        words[11] = bounce.to_bits();
        compile(&mut words, 1.0, [2.0; 3], [1.0; 3]);
        assert_eq!(
            read(&words, 6),
            [expected, 2.0, -3.0, sep],
            "sep {sep}, velocity {vel}, force {force}"
        );
    }
}

#[test]
fn axes_project_without_normalization_and_scales_apply_by_constraint() {
    let mut words = origin();
    row(&mut words, 2, [2.0, 1.0, 0.0, 0.0]);
    row(&mut words, 3, [0.0, 3.0, 1.0, 0.75]);
    row(&mut words, 4, [1.0, 0.0, 4.0, 0.25]);
    row(&mut words, 5, [1.0, 2.0, 3.0, 0.0]);
    compile(&mut words, 1.0, [2.0; 3], [0.5, 0.25, 0.125]);
    assert_eq!(read(&words, 2), [1.0, 0.0, 0.125, 0.0]);
    assert_eq!(read(&words, 3), [0.5, 0.75, 0.0, 0.75]);
    assert_eq!(read(&words, 4), [0.0, 0.25, 0.5, 0.25]);
    assert_eq!(read(&words, 6), [2.0, 2.25, 1.625, 0.0]);
}

#[test]
fn unavailable_estimate_preserves_every_input_word() {
    let mut words = origin();
    words[23] = 0xfedcba98;
    row(&mut words, 5, [3.0, -2.0, 1.0, 0.0]);
    let before = words;
    let result = build_with_response(&mut words, 0.5, &mut UnavailableContactMassResponse);
    assert_eq!(
        result,
        Err(ContactBuildError::ReciprocalEstimateUnavailable {
            effective_mass: [2.0; 3]
        })
    );
    assert_eq!(words, before);
}

#[test]
fn prepared_rotated_contact_drives_shared_solver_reactions() {
    use skate_core::physics::solver::packed::{self, Contact};
    let mut words = origin();
    // Normal Y, tangents Z/X. Only A has a lever arm, +X.
    row(&mut words, 0, [1.0, 0.0, 0.0, 0.0]);
    row(&mut words, 1, [1.0, 2.0, 0.0, 0.0]);
    row(&mut words, 7, [1.0, 2.0, 0.0, 0.0]);
    row(&mut words, 2, [0.0, 1.0, 0.0, 0.0]);
    row(&mut words, 3, [0.0, 0.0, 1.0, 0.75]);
    row(&mut words, 4, [1.0, 0.0, 0.0, 0.25]);
    row(&mut words, 5, [8.0, 0.0, 4.0, 0.0]);
    // Deliberately explicit algebraic scales, not a reciprocal approximation.
    compile(&mut words, 1.0, [3.0, 3.0, 2.0], [0.25, 0.5, 0.125]);
    let mut contacts = [Contact {
        words,
        reaction_a: 0,
        reaction_b: 1,
    }];
    let mut reactions = [[0; 16]; 2];
    packed::solve(&mut contacts, &mut [], &mut [], &mut reactions, 2);
    // Pass one gives normal .5 and no friction. On pass two the normal
    // correction is -.25; friction uses old normal .5 and dynamic limit .125.
    assert_eq!(read(&contacts[0].words, 5), [0.25, 0.125, 0.125, 0.625]);
    let read_reaction = |body: usize, row: usize| -> [f32; 4] {
        core::array::from_fn(|lane| f32::from_bits(reactions[body][row * 4 + lane]))
    };
    assert_eq!(read_reaction(0, 0), [0.125, 0.25, 0.125, 0.0]);
    assert_eq!(read_reaction(1, 0), [-0.125, -0.25, -0.125, 0.0]);
    assert_eq!(read_reaction(0, 1), [0.0, 0.625, 0.0, 0.0]);
    assert_eq!(read_reaction(1, 1), [0.0, -0.625, 0.0, 0.0]);
    assert_eq!(read_reaction(0, 2), [0.0, -0.125, 0.25, 0.125]);
    assert_eq!(read_reaction(1, 2), [0.0, 0.0, 0.0, -0.25]);
    assert_eq!(read_reaction(0, 3), [0.0, 0.0, 0.625, 0.625]);
    assert_eq!(read_reaction(1, 3), [0.0, 0.0, 0.0, -0.625]);
}
