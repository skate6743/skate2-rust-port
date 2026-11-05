//! Independent geometric/constructor checks; no generated execution fixtures.
use skate_core::{
    math::Vector3,
    physics::mass::{
        MassShape, PartMassInput, TruckMassSettings, WheelMassSettings, forward_mass_properties,
        primitive_mass, truck_mass_input, wheel_mass_input,
    },
};

fn close(actual: f32, expected: f64) {
    assert!(
        (f64::from(actual) - expected).abs() <= expected.abs().max(1e-25) * 3e-6,
        "actual {actual:?}, expected {expected:?}"
    );
}

#[test]
fn sphere_obeys_solid_sphere_moments_and_volume() {
    let mass = primitive_mass(MassShape::Sphere { radius: 2.0 }).unwrap();
    for moment in [
        mass.moments_per_unit_mass.x,
        mass.moments_per_unit_mass.y,
        mass.moments_per_unit_mass.z,
    ] {
        close(moment, 8.0 / 5.0);
    }
    close(mass.volume, 32.0 * std::f64::consts::PI / 3.0);
}

#[test]
fn capsule_uses_native_longitudinal_axis_and_dimension_floor() {
    let mass = primitive_mass(MassShape::Capsule {
        radius: 1.0,
        half_length: 2.0,
    })
    .unwrap();
    close(mass.moments_per_unit_mass.x, 271.0 / 100.0);
    assert_eq!(mass.moments_per_unit_mass.x, mass.moments_per_unit_mass.y);
    close(mass.moments_per_unit_mass.z, 2.0 / 5.0);
    close(mass.volume, 16.0 * std::f64::consts::PI / 3.0);

    let floor = primitive_mass(MassShape::Capsule {
        radius: 1.0,
        half_length: 20.0,
    });
    for radius in [-2.0, 0.0, 0.5, 1.0] {
        assert_eq!(
            primitive_mass(MassShape::Capsule {
                radius,
                half_length: 20.0
            }),
            floor
        );
    }
    assert_ne!(
        primitive_mass(MassShape::Capsule {
            radius: 1.5,
            half_length: 20.0
        }),
        floor
    );
}

#[test]
fn sphere_radius_floor_includes_negative_zero_and_unordered_input() {
    let minimum = f32::from_bits(0x33d6_bf95);
    let floor = primitive_mass(MassShape::Sphere { radius: minimum });
    for radius in [0.0, -0.0, -10.0, minimum * 0.5, f32::NAN, f32::NEG_INFINITY] {
        assert_eq!(primitive_mass(MassShape::Sphere { radius }), floor);
    }
    assert_ne!(
        primitive_mass(MassShape::Sphere {
            radius: minimum * 2.0
        }),
        floor
    );
}

#[test]
fn box_without_rounding_has_cuboid_properties() {
    let mass = primitive_mass(MassShape::RoundedBox {
        half_extents: Vector3::new(1.0, 2.0, 3.0),
        radius: 0.0,
    })
    .unwrap();
    close(mass.moments_per_unit_mass.x, 13.0 / 3.0);
    close(mass.moments_per_unit_mass.y, 10.0 / 3.0);
    close(mass.moments_per_unit_mass.z, 5.0 / 3.0);
    assert_eq!(mass.volume, 48.0);
}

#[test]
fn rounded_box_includes_edge_and_corner_volume() {
    let mass = primitive_mass(MassShape::RoundedBox {
        half_extents: Vector3::new(1.0, 2.0, 3.0),
        radius: 0.5,
    })
    .unwrap();
    close(mass.volume, 92.0 + 19.0 * std::f64::consts::PI / 6.0);
    close(mass.moments_per_unit_mass.x, 49.0 / 8.0);
    close(mass.moments_per_unit_mass.y, 115.0 / 24.0);
    close(mass.moments_per_unit_mass.z, 67.0 / 24.0);
}

#[test]
fn box_floor_depends_on_largest_padded_dimension() {
    for half_extents in [
        Vector3::new(0.0, 0.0, 20.0),
        Vector3::new(0.0, 20.0, 0.0),
        Vector3::new(20.0, 0.0, 0.0),
    ] {
        let mass = primitive_mass(MassShape::RoundedBox {
            half_extents,
            radius: 0.0,
        })
        .unwrap();
        assert_eq!(mass.volume, 160.0);
    }
    let thin = primitive_mass(MassShape::RoundedBox {
        half_extents: Vector3::new(-3.0, 0.0, 20.0),
        radius: 0.5,
    });
    let floored = primitive_mass(MassShape::RoundedBox {
        half_extents: Vector3::new(0.525, 0.525, 20.0),
        radius: 0.5,
    });
    assert_eq!(thin, floored);
}

#[test]
fn cylinder_padding_expands_radius_and_half_length() {
    let bare = primitive_mass(MassShape::Cylinder {
        radius: 3.0,
        half_length: 2.0,
        padding: 0.0,
    })
    .unwrap();
    let padded = primitive_mass(MassShape::Cylinder {
        radius: 2.5,
        half_length: 1.5,
        padding: 0.5,
    })
    .unwrap();
    assert_eq!(bare, padded);
    close(bare.moments_per_unit_mass.x, 43.0 / 12.0);
    assert_eq!(bare.moments_per_unit_mass.z, 4.5);
    close(bare.volume, 36.0 * std::f64::consts::PI);
}

#[test]
fn geometric_scaling_is_quadratic_for_moments_and_cubic_for_volume() {
    let shapes = [
        (
            MassShape::Sphere { radius: 0.75 },
            MassShape::Sphere { radius: 1.5 },
        ),
        (
            MassShape::Capsule {
                radius: 0.75,
                half_length: 1.25,
            },
            MassShape::Capsule {
                radius: 1.5,
                half_length: 2.5,
            },
        ),
        (
            MassShape::RoundedBox {
                half_extents: Vector3::new(1.0, 2.0, 3.0),
                radius: 0.25,
            },
            MassShape::RoundedBox {
                half_extents: Vector3::new(2.0, 4.0, 6.0),
                radius: 0.5,
            },
        ),
        (
            MassShape::Cylinder {
                radius: 0.75,
                half_length: 1.25,
                padding: 0.25,
            },
            MassShape::Cylinder {
                radius: 1.5,
                half_length: 2.5,
                padding: 0.5,
            },
        ),
    ];
    for (small, large) in shapes {
        let small = primitive_mass(small).unwrap();
        let large = primitive_mass(large).unwrap();
        assert_eq!(large.volume, small.volume * 8.0);
        assert_eq!(
            large.moments_per_unit_mass,
            Vector3::new(
                small.moments_per_unit_mass.x * 4.0,
                small.moments_per_unit_mass.y * 4.0,
                small.moments_per_unit_mass.z * 4.0,
            )
        );
    }
}

#[test]
fn constructors_derive_geometry_and_scale_requested_mass() {
    let wheel = wheel_mass_input(WheelMassSettings {
        radius: 0.125,
        mass: 0.25,
        mass_factor: 2.0,
    });
    assert_eq!(
        wheel,
        PartMassInput {
            shape: MassShape::Sphere { radius: 0.125 },
            requested_mass: 0.5
        }
    );
    let truck = truck_mass_input(TruckMassSettings {
        wheel_radius: 0.125,
        wheel_x_distance: 0.25,
        radius_scalar: 2.0,
        half_height_scalar: 3.0,
        mass: 0.5,
        mass_factor: 4.0,
    });
    assert_eq!(
        truck,
        PartMassInput {
            shape: MassShape::Capsule {
                radius: 0.125,
                half_length: 0.375
            },
            requested_mass: 2.0,
        }
    );
    let stock = forward_mass_properties(wheel_mass_input(WheelMassSettings::STOCK)).unwrap();
    assert_eq!(stock.mass.to_bits(), 0x3ed4_7ae1); // 0.415
    close(stock.principal_moments.x, 0.4 * 0.031_f64.powi(2) * 0.415);
    let truck = truck_mass_input(TruckMassSettings::STOCK);
    if let MassShape::Capsule {
        radius,
        half_length,
    } = truck.shape
    {
        close(radius, 0.00833125);
        close(half_length, 0.064125);
    } else {
        panic!("native truck producer must use capsule");
    }
    assert_eq!(truck.requested_mass.to_bits(), 0x3ff9_9999); // 1.95
}

#[test]
fn mass_fallback_is_strictly_below_minimum_normal_and_preserves_nan() {
    let shape = MassShape::Sphere { radius: 1.0 };
    let volume = primitive_mass(shape).unwrap().volume;
    for requested_mass in [0.0, -1.0, f32::from_bits(1), f32::MIN_POSITIVE * 0.5] {
        assert_eq!(
            forward_mass_properties(PartMassInput {
                shape,
                requested_mass
            })
            .unwrap()
            .mass,
            volume
        );
    }
    for requested_mass in [f32::MIN_POSITIVE, 2.0, f32::INFINITY] {
        assert_eq!(
            forward_mass_properties(PartMassInput {
                shape,
                requested_mass
            })
            .unwrap()
            .mass,
            requested_mass
        );
    }
    assert!(
        forward_mass_properties(PartMassInput {
            shape,
            requested_mass: f32::NAN
        })
        .unwrap()
        .mass
        .is_nan()
    );
    assert_eq!(primitive_mass(MassShape::Unsupported), None);
    assert_eq!(
        forward_mass_properties(PartMassInput {
            shape: MassShape::Unsupported,
            requested_mass: 1.0
        }),
        None
    );
}
