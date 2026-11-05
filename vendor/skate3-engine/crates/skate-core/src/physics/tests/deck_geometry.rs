use super::{DeckChild, DeckGeometry, DeckGeometrySettings, DeckShape, MassMoments};
use crate::{math::Vector3, physics::drive_frames::RetailAffineTransform};

fn close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected}"
    );
}

#[test]
fn stock_deck_has_actual_child_order_dimensions_and_source_flags() {
    let deck = DeckGeometry::new(DeckGeometrySettings::STOCK);
    assert_eq!(deck.children.len(), 15);
    let DeckShape::RoundedBox {
        half_extents,
        radius,
    } = deck.children[0].shape
    else {
        panic!()
    };
    close(radius, 0.00675, 1e-9);
    close(half_extents.x + radius, 0.12, 1e-8);
    close(half_extents.y + radius, 0.0075, 1e-9);
    close(half_extents.z + radius, 0.295, 3e-8);
    for (index, x) in [(1, 0.1125), (2, -0.1125)] {
        let DeckShape::Capsule {
            radius,
            half_length,
        } = deck.children[index].shape
        else {
            panic!()
        };
        close(radius, 0.0075, 1e-9);
        close(half_length, 0.295, 3e-8);
        close(deck.children[index].transform.translation.x, x, 1e-8);
        assert_eq!(
            deck.children[index].transform.basis,
            RetailAffineTransform::IDENTITY.basis
        );
    }
    for (index, z) in [(3, 0.235), (4, -0.235)] {
        assert_eq!(
            deck.children[index].shape,
            DeckShape::Sphere { radius: 0.035 }
        );
        assert_eq!(
            deck.children[index].transform.translation,
            Vector3::new(0.0, -0.02, z)
        );
    }
    for (index, child) in deck.children.iter().enumerate().skip(5) {
        let DeckShape::Triangle {
            vertices,
            fatness,
            edge_cosines,
            volume_flags,
        } = child.shape
        else {
            panic!()
        };
        assert_eq!(fatness, 0.0075);
        assert_eq!(edge_cosines, [1.0, -1.0, 1.0]);
        assert_eq!(volume_flags, 0x3E2);
        assert_eq!(
            vertices[0],
            Vector3::new(0.0, 0.0, if index % 2 == 1 { -0.295 } else { 0.295 })
        );
    }
}

#[test]
fn fans_join_at_shared_vertices_and_use_independent_end_angles() {
    let deck = DeckGeometry::new(DeckGeometrySettings::STOCK);
    for side in [0, 1] {
        let triangles: Vec<_> = deck.children[5 + side..]
            .iter()
            .step_by(2)
            .map(|child| {
                let DeckShape::Triangle { vertices, .. } = child.shape else {
                    panic!()
                };
                vertices
            })
            .collect();
        for pair in triangles.windows(2) {
            assert_eq!(pair[0][2], pair[1][1]);
        }
        close(triangles[0][1].x, 0.1125, 1e-8);
        close(triangles[4][2].x, -0.1125, 2e-7);
        for vertices in triangles {
            assert!(vertices[2].y >= -1e-7);
            let from_center = (vertices[2].z - vertices[0].z).abs();
            if from_center > 0.01 {
                let actual_angle = (vertices[2].y / from_center).atan().to_degrees();
                close(actual_angle, if side == 0 { 12.5 } else { 13.0 }, 0.0001);
            }
        }
    }
}

#[test]
fn collision_toggles_do_not_remove_mass_and_spheres_remain_enabled() {
    let stock = DeckGeometry::new(DeckGeometrySettings::STOCK);
    let mut settings = DeckGeometrySettings::STOCK;
    settings.enable_deck_volume_collisions = false;
    settings.enable_end_volume_collisions = false;
    let disabled = DeckGeometry::new(settings);
    assert_eq!(stock.mass_moments(), disabled.mass_moments());
    for (index, child) in disabled.children.iter().enumerate() {
        assert_eq!(child.collision_enabled, index == 3 || index == 4);
    }
}

#[test]
fn triangle_mass_uses_source_fat_aabb_instead_of_triangle_area() {
    let child = DeckChild {
        shape: DeckShape::Triangle {
            vertices: [
                Vector3::new(-2.0, 1.0, -1.0),
                Vector3::new(3.0, 1.0, -1.0),
                Vector3::new(1.0, 1.0, 5.0),
            ],
            fatness: 0.5,
            edge_cosines: [1.0, -1.0, 1.0],
            volume_flags: 0x3E2,
        },
        transform: RetailAffineTransform::IDENTITY,
        collision_enabled: true,
    };
    let mut moments = child.mass_moments();
    let properties = moments.principal_properties();
    // Its fat AABB is [-2.5,0.5,-1.5]..[3.5,1.5,5.5]. The source rounded-box
    // minimum radius is tiny but nonzero; allow its documented numerical floor.
    close(properties.volume, 42.0, 0.0001);
    close(properties.local_mass_frame.translation.x, 0.5, 2e-6);
    close(properties.local_mass_frame.translation.y, 1.0, 2e-6);
    close(properties.local_mass_frame.translation.z, 2.0, 2e-6);
}

#[test]
fn stock_mass_is_computed_from_children_and_changes_with_authored_dimensions() {
    let stock = DeckGeometry::new(DeckGeometrySettings::STOCK);
    let mut moments = stock.mass_moments();
    assert_ne!(moments, MassMoments::ZERO);
    let properties = moments.principal_properties();
    assert!(properties.volume > 0.0);
    close(properties.local_mass_frame.translation.x, 0.0, 1e-6);
    assert!(properties.local_mass_frame.translation.y.is_finite());
    let inertia = properties.moments_per_unit_mass;
    assert!(inertia.x > 0.0 && inertia.y > 0.0 && inertia.z > 0.0);
    let mut settings = DeckGeometrySettings::STOCK;
    settings.back_end_size *= 1.5;
    let extended = DeckGeometry::new(settings);
    let mut extended_moments = extended.mass_moments();
    let extended_properties = extended_moments.principal_properties();
    assert!(extended_properties.volume > properties.volume);
    assert_ne!(extended_moments, moments);
    for index in 0..5 {
        assert_eq!(stock.children[index], extended.children[index]);
    }
    for index in 5..15 {
        assert_ne!(stock.children[index], extended.children[index]);
    }
}

#[test]
fn authored_count_controls_both_end_fans() {
    let mut settings = DeckGeometrySettings::STOCK;
    settings.end_capsule_count = 3;
    assert_eq!(DeckGeometry::new(settings).children.len(), 11);
    settings.end_capsule_count = 0;
    assert_eq!(DeckGeometry::new(settings).children.len(), 5);
}
