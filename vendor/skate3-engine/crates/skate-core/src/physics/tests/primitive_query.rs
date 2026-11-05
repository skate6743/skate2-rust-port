use crate::{
    math::{Basis3, Vector3},
    physics::{
        collision::{
            Sphere, Triangle, TriangleFeature, WorldContactSettings, sphere_triangle_world_contact,
        },
        world_contact::{
            ContactPrimitive, primitive_triangle_world_contacts, transform_triangle_volume,
            triangle_from_volume,
        },
    },
};

fn v(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z)
}
fn floor() -> Triangle {
    triangle_from_volume(
        [v(-5.0, 0.0, -5.0), v(0.0, 0.0, 5.0), v(5.0, 0.0, -5.0)],
        0.0,
        [-1.0; 3],
        2,
    )
}
fn settings(padding: f32) -> WorldContactSettings {
    WorldContactSettings {
        volume_padding: padding,
        maximum_separating_distance: 1.0,
        edge_cos_bend_normal_threshold: -1.0,
        convexity_epsilon: 0.0,
        is_object: false,
    }
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 2.0e-5, "{a} != {b}");
}
fn same(a: Vector3, b: Vector3) {
    close(a.x, b.x);
    close(a.y, b.y);
    close(a.z, b.z);
}
fn identity() -> Basis3 {
    Basis3 {
        columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    }
}

#[test]
fn volume_builder_uses_native_winding_edges_and_flags() {
    let triangle = triangle_from_volume(
        [v(0.0, 0.0, 0.0), v(0.0, 0.0, 4.0), v(3.0, 0.0, 0.0)],
        0.02,
        [1.0, -1.0, 1.0],
        0x3e3,
    );
    same(triangle.feature.normal, v(0.0, 1.0, 0.0));
    same(triangle.feature.edges[0], v(1.0, 0.0, 0.0));
    same(triangle.feature.edges[1], v(-0.6, 0.0, 0.8));
    same(triangle.feature.edges[2], v(0.0, 0.0, -1.0));
    for (actual, expected) in triangle.edge_lengths.into_iter().zip([3.0, 5.0, 4.0]) {
        close(actual, expected);
    }
    assert_eq!(triangle.feature.flags, 0x3e1);
    assert_eq!(triangle.feature.edge_cosines, [1.0, -1.0, 1.0]);
    assert_eq!(triangle.fatness, 0.02);
}

#[test]
fn volume_transform_keeps_local_normal_and_rebuilds_world_edges() {
    let triangle = transform_triangle_volume(
        [v(0.0, 0.0, 0.0), v(0.0, 0.0, 4.0), v(3.0, 0.0, 0.0)],
        0.02,
        [-1.0; 3],
        2,
        Basis3 {
            columns: [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        },
        v(4.0, 5.0, 6.0),
    );
    same(triangle.feature.normal, v(-1.0, 0.0, 0.0));
    same(triangle.vertices[2], v(4.0, 8.0, 6.0));
    same(triangle.feature.edges[0], v(0.0, 1.0, 0.0));
    close(triangle.edge_lengths[1], 5.0);
}

#[test]
fn generic_sphere_world_path_matches_existing_independent_sphere_branch() {
    let sphere = Sphere {
        center: v(0.4, 0.13, -0.2),
        radius: 0.15,
    };
    let mut triangle = floor();
    triangle.fatness = 0.01;
    let generic = primitive_triangle_world_contacts(
        ContactPrimitive::Sphere(sphere),
        triangle,
        Vector3::ZERO,
        settings(0.0),
    )
    .unwrap();
    let previous =
        sphere_triangle_world_contact(sphere, triangle, Vector3::ZERO, settings(0.0)).unwrap();
    assert_eq!(generic.count, 1);
    same(generic.normal, previous.normal);
    same(generic.points[0].a, previous.points.a);
    same(generic.points[0].b, previous.points.b);
    same(generic.normal, v(0.0, 1.0, 0.0));
    close(generic.points[0].a.y, -0.02);
    close(generic.points[0].b.y, 0.01);
}

#[test]
fn capsule_face_keeps_both_axial_end_contacts() {
    let capsule = ContactPrimitive::Capsule {
        center: v(0.0, 0.1, 0.0),
        axis: v(1.0, 0.0, 0.0),
        half_length: 0.4,
        radius: 0.15,
    };
    let result =
        primitive_triangle_world_contacts(capsule, floor(), Vector3::ZERO, settings(0.0)).unwrap();
    assert_eq!(result.count, 2);
    same(result.normal, v(0.0, 1.0, 0.0));
    let mut xs = result.points[..2].iter().map(|p| p.a.x).collect::<Vec<_>>();
    xs.sort_by(f32::total_cmp);
    close(xs[0], -0.4);
    close(xs[1], 0.4);
    for p in &result.points[..2] {
        close(p.a.y, -0.05);
        close(p.b.y, 0.0);
        close(p.a.x, p.b.x);
    }
}

#[test]
fn rounded_box_face_produces_all_four_corner_contacts() {
    let shape = ContactPrimitive::RoundedBox {
        center: v(0.0, 0.09, 0.0),
        basis: identity(),
        half_extents: v(0.4, 0.08, 0.2),
        radius: 0.02,
    };
    let result =
        primitive_triangle_world_contacts(shape, floor(), Vector3::ZERO, settings(0.0)).unwrap();
    assert_eq!(result.count, 4);
    same(result.normal, v(0.0, 1.0, 0.0));
    for p in &result.points[..result.count] {
        close(p.a.y, -0.01);
        close(p.b.y, 0.0);
        close(p.a.x.abs(), 0.4);
        close(p.a.z.abs(), 0.2);
    }
}

#[test]
fn moving_triangle_face_preserves_three_vertex_contacts() {
    let moving = triangle_from_volume(
        [v(-0.4, 0.01, -0.2), v(0.0, 0.01, 0.2), v(0.4, 0.01, -0.2)],
        0.02,
        [-1.0; 3],
        2,
    );
    let result = primitive_triangle_world_contacts(
        ContactPrimitive::Triangle(moving),
        floor(),
        Vector3::ZERO,
        settings(0.0),
    )
    .unwrap();
    assert_eq!(result.count, 3);
    same(result.normal, v(0.0, 1.0, 0.0));
    for p in &result.points[..3] {
        close(p.a.y, -0.01);
        close(p.b.y, 0.0);
    }
}

#[test]
fn prediction_uses_fixed_sixtieth_and_separate_volume_padding() {
    let sphere = ContactPrimitive::Sphere(Sphere {
        center: v(0.0, 0.2, 0.0),
        radius: 0.1,
    });
    assert!(
        primitive_triangle_world_contacts(sphere, floor(), Vector3::ZERO, settings(0.0)).is_none()
    );
    let predicted =
        primitive_triangle_world_contacts(sphere, floor(), v(0.0, -6.0, 0.0), settings(0.001))
            .unwrap();
    close(predicted.points[0].a.y, 0.1);
    assert!(
        primitive_triangle_world_contacts(sphere, floor(), v(0.0, 6.0, 0.0), settings(0.001))
            .is_none()
    );
}

#[test]
fn one_sided_world_triangle_rejects_below_surface() {
    let mut triangle = floor();
    triangle.feature.flags |= TriangleFeature::ONE_SIDED;
    let below = ContactPrimitive::Sphere(Sphere {
        center: v(0.0, -0.05, 0.0),
        radius: 0.1,
    });
    assert!(
        primitive_triangle_world_contacts(below, triangle, Vector3::ZERO, settings(0.0)).is_none()
    );
    let above = ContactPrimitive::Sphere(Sphere {
        center: v(0.0, 0.05, 0.0),
        radius: 0.1,
    });
    assert!(
        primitive_triangle_world_contacts(above, triangle, Vector3::ZERO, settings(0.0)).is_some()
    );
}

#[test]
fn triangle_box_edge_cross_axis_rejects_despite_overlapping_face_intervals() {
    // All three box-axis intervals and the triangle normal overlap. The
    // diagonal triangle edge x+z=2.1 is sqrt(0.005) beyond box corner(1,1),
    // so this separation can only be found by an edge-cross SAT direction.
    let triangle = triangle_from_volume(
        [v(0.0, 0.0, 2.1), v(3.0, 0.0, 3.0), v(2.1, 0.0, 0.0)],
        0.0,
        [-1.0; 3],
        0xe2,
    );
    let shape = |radius| ContactPrimitive::RoundedBox {
        center: Vector3::ZERO,
        basis: identity(),
        half_extents: v(1.0, 1.0, 1.0),
        radius,
    };
    assert!(
        primitive_triangle_world_contacts(shape(0.05), triangle, Vector3::ZERO, settings(0.0))
            .is_none()
    );
    let result =
        primitive_triangle_world_contacts(shape(0.08), triangle, Vector3::ZERO, settings(0.0))
            .unwrap();
    assert_eq!(result.count, 1);
    same(
        result.normal,
        v(
            -std::f32::consts::FRAC_1_SQRT_2,
            0.0,
            -std::f32::consts::FRAC_1_SQRT_2,
        ),
    );
    same(result.points[0].b, v(1.05, 0.0, 1.05));
    let offset = 0.08 * std::f32::consts::FRAC_1_SQRT_2;
    same(result.points[0].a, v(1.0 + offset, 0.0, 1.0 + offset));
}

#[test]
fn rotated_box_and_triangle_keep_the_plane_manifold() {
    let c = 0.8660254;
    let basis = Basis3 {
        columns: [[c, 0.5, 0.0], [-0.5, c, 0.0], [0.0, 0.0, 1.0]],
    };
    let triangle =
        transform_triangle_volume(floor().vertices, 0.0, [-1.0; 3], 2, basis, v(2.0, 3.0, 4.0));
    let shape = ContactPrimitive::RoundedBox {
        center: v(2.0 - 0.09 * 0.5, 3.0 + 0.09 * c, 4.0),
        basis,
        half_extents: v(0.4, 0.08, 0.2),
        radius: 0.02,
    };
    let result =
        primitive_triangle_world_contacts(shape, triangle, Vector3::ZERO, settings(0.0)).unwrap();
    assert_eq!(result.count, 4);
    same(result.normal, v(-0.5, c, 0.0));
    for p in &result.points[..result.count] {
        close((p.a.x - 2.0) * -0.5 + (p.a.y - 3.0) * c, -0.01);
        close((p.b.x - 2.0) * -0.5 + (p.b.y - 3.0) * c, 0.0);
    }
}
