use crate::{
    math::{Basis3, Vector3},
    physics::mass::{MassMoments, PrimitiveMass},
};

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 2.0e-5, "{actual} != {expected}");
}

#[test]
fn translated_primitive_recovers_center_without_parallel_axis_inertia() {
    let mut moments = MassMoments::from_primitive(PrimitiveMass {
        moments_per_unit_mass: Vector3::new(2.0, 3.0, 4.0),
        volume: 2.0,
    });
    let identity = Basis3 {
        columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };
    moments.transform(identity, Vector3::new(2.0, -1.0, 3.0));
    let result = moments.principal_properties();
    assert_eq!(
        result.local_mass_frame.translation,
        Vector3::new(2.0, -1.0, 3.0)
    );
    close(result.moments_per_unit_mass.x, 2.0);
    close(result.moments_per_unit_mass.y, 3.0);
    close(result.moments_per_unit_mass.z, 4.0);
}

#[test]
fn rotated_mass_reconstructs_independent_world_inertia() {
    let sine = 0.6;
    let cosine = 0.8;
    let basis = Basis3 {
        columns: [[cosine, sine, 0.0], [-sine, cosine, 0.0], [0.0, 0.0, 1.0]],
    };
    let mut moments = MassMoments::from_primitive(PrimitiveMass {
        moments_per_unit_mass: Vector3::new(2.0, 3.0, 4.0),
        volume: 2.0,
    });
    moments.transform(basis, Vector3::new(0.0, 0.0, 0.0));
    let result = moments.principal_properties();
    let columns = result.local_mass_frame.basis.columns;
    let diagonal = [
        result.moments_per_unit_mass.x,
        result.moments_per_unit_mass.y,
        result.moments_per_unit_mass.z,
    ];
    // R diag(2,3,4) R^T for the independently specified rotation.
    let expected = [[2.36, -0.48, 0.0], [-0.48, 2.64, 0.0], [0.0, 0.0, 4.0]];
    for row in 0..3 {
        for column in 0..3 {
            let actual: f32 = (0..3)
                .map(|axis| columns[axis][row] * diagonal[axis] * columns[axis][column])
                .sum();
            close(actual, expected[row][column]);
        }
    }
}

#[test]
fn two_translated_equal_volumes_preserve_parallel_axis_contribution() {
    let primitive = PrimitiveMass {
        moments_per_unit_mass: Vector3::new(1.0, 1.0, 1.0),
        volume: 3.0,
    };
    let identity = Basis3 {
        columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };
    let mut total = MassMoments::ZERO;
    for x in [-2.0, 2.0] {
        let mut child = MassMoments::from_primitive(primitive);
        child.transform(identity, Vector3::new(x, 0.0, 0.0));
        total.add(child);
    }
    let result = total.principal_properties();
    assert_eq!(result.volume, 6.0);
    close(result.moments_per_unit_mass.x, 1.0);
    close(result.moments_per_unit_mass.y, 5.0);
    close(result.moments_per_unit_mass.z, 5.0);
}

#[test]
fn aggregate_finalizer_stores_inverse_frame_and_keeps_world_inertia() {
    use crate::physics::mass::aggregate_mass_properties;
    let rotation = Basis3 {
        columns: [[0.8, 0.6, 0.0], [-0.6, 0.8, 0.0], [0.0, 0.0, 1.0]],
    };
    let center = Vector3::new(2.0, -1.0, 3.0);
    let mut moments = MassMoments::from_primitive(PrimitiveMass {
        moments_per_unit_mass: Vector3::new(2.0, 3.0, 4.0),
        volume: 2.0,
    });
    moments.transform(rotation, center);
    let result = aggregate_mass_properties(moments, 2.0, 20.0, 0.25);
    let inverse = result.local_mass_frame;
    let translation = [
        inverse.translation.x,
        inverse.translation.y,
        inverse.translation.z,
    ];
    for row in 0..3 {
        // Applying inverseBodyLTM to the authored COM must produce the origin.
        let local_center = inverse.basis.columns[0][row] * center.x
            + inverse.basis.columns[1][row] * center.y
            + inverse.basis.columns[2][row] * center.z
            + translation[row];
        close(local_center, 0.0);
    }
    let tensor = result.dynamics.inverse_tensor;
    let principal = [1.0 / tensor.x, 1.0 / tensor.y, 1.0 / tensor.z];
    // I_world = transpose(inverse_basis) * I_principal * inverse_basis.
    let expected = [[4.72, -0.96, 0.0], [-0.96, 5.28, 0.0], [0.0, 0.0, 8.0]];
    for row in 0..3 {
        for column in 0..3 {
            let actual: f32 = (0..3)
                .map(|axis| {
                    inverse.basis.columns[row][axis]
                        * principal[axis]
                        * inverse.basis.columns[column][axis]
                })
                .sum();
            close(actual, expected[row][column]);
        }
    }
}

#[test]
fn finalizer_omits_source_negligible_frame_correction() {
    use crate::physics::{mass::aggregate_mass_properties, rigid_body::RetailLocalMassFrame};
    let mut moments = MassMoments::from_primitive(PrimitiveMass {
        moments_per_unit_mass: Vector3::new(2.0, 3.0, 4.0),
        volume: 2.0,
    });
    moments.transform(
        RetailLocalMassFrame::IDENTITY.basis,
        Vector3::new(0.0001, 0.0, 0.0),
    );
    let result = aggregate_mass_properties(moments, 2.0, 20.0, 0.0);
    assert_eq!(result.local_mass_frame, RetailLocalMassFrame::IDENTITY);
}
