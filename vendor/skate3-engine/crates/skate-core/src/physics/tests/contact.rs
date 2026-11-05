use super::*;

fn body(contact_body_id: u32, center_of_mass: Vector3) -> RetailContactBodyState {
    RetailContactBodyState {
        contact_body_id,
        center_of_mass,
        reaction_id: contact_body_id + 10,
        inverse_inertia_full: Vector3::new(1.0, 2.0, 3.0),
        inverse_mass: 0.25,
        inverse_inertia_split: Vector3::new(4.0, 5.0, 6.0),
        state: 7,
        force_acceleration: Vector3::new(8.0, 9.0, 10.0),
        kinetic_energy: 11.0,
        torque_acceleration: Vector3::new(12.0, 13.0, 14.0),
        cool_down: 15,
        linear_velocity: Vector3::ZERO,
        angular_velocity: Vector3::ZERO,
    }
}

fn input(normal: Vector3) -> RetailContactInput {
    RetailContactInput {
        position_on_a: Vector3::new(2.0, 3.0, 4.0),
        position_on_b: Vector3::new(2.0, 3.0, 4.0),
        normal,
        restitution: 0.1,
        static_friction: 0.8,
        dynamic_friction: 0.7,
        tag: 0x1234_5678,
    }
}

#[test]
fn material_combine_matches_tu3_branch_directions() {
    let combined = combine_contact_materials(
        RetailContactMaterial {
            static_friction: f32::from_bits(0x3F4C_CCCD),
            dynamic_friction: f32::from_bits(0x3F33_3333),
            restitution: f32::from_bits(0x3E4C_CCCD),
        },
        RetailContactMaterial {
            static_friction: f32::from_bits(0x3F00_0000),
            dynamic_friction: f32::from_bits(0x3F66_6666),
            restitution: f32::from_bits(0x3DCC_CCCD),
        },
    );

    assert_eq!(combined.static_friction.to_bits(), 0x3F4C_CCCD);
    assert_eq!(combined.dynamic_friction.to_bits(), 0x3F66_6666);
    assert_eq!(combined.restitution.to_bits(), 0x3DCC_CCCD);
}

#[test]
fn material_combine_is_commutative_for_finite_values() {
    let a = RetailContactMaterial {
        static_friction: 0.8,
        dynamic_friction: 0.7,
        restitution: 0.0,
    };
    let b = RetailContactMaterial {
        static_friction: 0.5,
        dynamic_friction: 0.9,
        restitution: 0.2,
    };

    assert_eq!(
        combine_contact_materials(a, b),
        combine_contact_materials(b, a)
    );
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-6,
        "actual {actual}, expected {expected}"
    );
}

fn assert_vec_close(actual: Vector3, expected: Vector3) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.z, expected.z);
}

#[test]
fn angular_contact_rates_use_omega_cross_arm() {
    let mut body_a = body(1, Vector3::new(1.0, 3.0, 4.0));
    body_a.angular_velocity = Vector3::new(0.0, 0.0, 2.0);
    let contact = generate_contact(
        input(Vector3::new(0.0, -1.0, 0.0)),
        body_a,
        body(2, Vector3::new(2.0, 3.0, 4.0)),
    );

    assert_vec_close(contact.relative_velocity, Vector3::new(0.0, -2.0, 0.0));
}

#[test]
fn contact_frame_is_orthogonal() {
    let normal = Vector3::new(0.36, 0.48, 0.8);
    let mut body_b = body(2, Vector3::ZERO);
    body_b.linear_velocity = Vector3::new(2.0, -1.0, 0.5);
    let contact = generate_contact(input(normal), body(1, Vector3::ZERO), body_b);

    assert_close(dot(contact.normal, contact.tangent_0), 0.0);
    assert_close(dot(contact.normal, contact.tangent_1), 0.0);
    assert_close(dot(contact.tangent_0, contact.tangent_1), 0.0);
    assert_vec_close(cross(contact.normal, contact.tangent_0), contact.tangent_1);
}

#[test]
fn contact_copies_header_ids_materials_tag_and_body_workspaces() {
    let body_a = body(41, Vector3::new(1.0, 2.0, 3.0));
    let body_b = body(52, Vector3::new(4.0, 5.0, 6.0));
    let collision = input(Vector3::new(0.0, 1.0, 0.0));
    let contact = generate_contact(collision, body_a, body_b);

    assert_eq!(contact.body_a_id, 41);
    assert_eq!(contact.body_b_id, 52);
    assert_eq!(contact.restitution, collision.restitution);
    assert_eq!(contact.static_friction, collision.static_friction);
    assert_eq!(contact.dynamic_friction, collision.dynamic_friction);
    assert_eq!(contact.tag, collision.tag);
    assert_eq!(contact.body_a_workspace, body_a.into());
    assert_eq!(contact.body_b_workspace, body_b.into());
    assert_eq!(contact.body_a_workspace.reaction_id, 51);
    assert_eq!(contact.body_b_workspace.reaction_id, 62);
}

#[test]
fn recovered_tangent_constants_keep_their_exact_binary32_bits() {
    assert_eq!(CONTACT_TANGENT_MINIMUM_SQUARED.to_bits(), 0x0080_0000);
    assert_eq!(CONTACT_FALLBACK_HALF.to_bits(), 0x3F00_0000);
}
