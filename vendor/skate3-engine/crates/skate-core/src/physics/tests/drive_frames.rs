use super::*;

#[test]
fn identity_pair_has_bit_exact_identity_matrix_stage() {
    let parent_frame = relative_parent_transform(
        RetailAffineTransform::IDENTITY,
        RetailAffineTransform::IDENTITY,
    );

    assert_eq!(
        basis_bits(parent_frame.basis),
        [
            [0x3F80_0000, 0, 0],
            [0, 0x3F80_0000, 0],
            [0, 0, 0x3F80_0000],
        ]
    );
    assert_eq!(vector_bits(parent_frame.translation), [0; 3]);
}

#[test]
fn default_truck_transforms_match_tu3_stock_finite_path_bits() {
    let [slot_7712, slot_7776] = default_truck_transforms();

    assert_eq!(
        basis_bits(slot_7712.basis),
        [
            [0xB34B_C7FA, 0x3F03_D987, 0xBF5B_6F50],
            [0, 0x3F5B_6F51, 0x3F03_D988],
            [0x3F7F_FFFF, 0x32D1_E8FB, 0xB32E_ACAF],
        ]
    );
    assert_eq!(
        vector_bits(slot_7712.translation),
        [0, 0xBD67_6C8B, 0xBE78_D4FD,]
    );

    assert_eq!(
        basis_bits(slot_7776.basis),
        [
            [0xB34B_C7FA, 0x3F03_D987, 0x3F5B_6F50],
            [0, 0x3F5B_6F51, 0xBF03_D988],
            [0xBF7F_FFFF, 0xB2D1_E8FB, 0xB32E_ACAF],
        ]
    );
    assert_eq!(
        vector_bits(slot_7776.translation),
        [0, 0xBD67_6C8B, 0x3E78_D4FD,]
    );
}

#[test]
fn matrix_stage_preserves_tu3_vmx_fused_order() {
    let [slot_7712, slot_7776] = default_truck_transforms();
    let relative = relative_parent_transform(slot_7712, slot_7776);
    assert_eq!(
        basis_bits(relative.basis),
        [
            [0xBEF0_5E95, 0x3F62_08D7, 0xB395_B9AE],
            [0x3F62_08D7, 0x3EF0_5E96, 0x3333_ED95],
            [0x3395_B9AE, 0xB333_ED95, 0xBF7F_FFFE],
        ]
    );

    // This finite vector distinguishes TU3's six-stage translation
    // accumulation from pre-subtracting the two translations before a dot.
    let parent_translation = vector_from_bits([0x43D5_62ED, 0xC356_E21F, 0x4408_E252]);
    let child_translation = vector_from_bits([0x43FA_50C2, 0xC28B_22BA, 0xC3E6_0F58]);
    let translation =
        transpose_relative_translation(slot_7712.basis, parent_translation, child_translation);
    assert_eq!(
        vector_bits(translation),
        [0x446A_A4A8, 0xC3C5_35F5, 0x4293_B75A,]
    );
}

#[test]
fn derived_level_spawn_places_all_four_wheel_spheres_on_y_zero() {
    let bodies = default_body_transforms();

    for wheel in &bodies[..4] {
        let wheel_bottom =
            RETAIL_LEVEL_GROUND_DECK_CENTER_HEIGHT + wheel.translation.y - RETAIL_WHEEL_RADIUS;
        assert!(wheel_bottom.abs() <= f32::EPSILON);
    }
}

fn basis_bits(basis: Basis3) -> [[u32; 3]; 3] {
    basis.columns.map(|column| column.map(f32::to_bits))
}

fn vector_bits(vector: Vector3) -> [u32; 3] {
    [vector.x.to_bits(), vector.y.to_bits(), vector.z.to_bits()]
}

fn vector_from_bits(bits: [u32; 3]) -> Vector3 {
    Vector3::new(
        f32::from_bits(bits[0]),
        f32::from_bits(bits[1]),
        f32::from_bits(bits[2]),
    )
}
