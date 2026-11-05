use super::*;

fn vertices() -> [Vector3; 3] {
    [
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(1.0, 0.0, 0.0),
    ]
}

#[test]
fn wheel_segment_preserves_winding_fraction_and_barycentric_coordinates() {
    let hit = thin_triangle(
        Vector3::new(0.25, 0.1, 0.25),
        Vector3::new(0.0, -0.2, 0.0),
        vertices(),
    )
    .unwrap();
    assert!((hit.fraction - 0.5).abs() < 1e-6);
    assert!(hit.position.y.abs() < 1e-7);
    assert!((hit.normal.y - 1.0).abs() < 1e-6);
    assert_eq!(hit.volume_parameter, [0.25, 0.25, 0.0]);
    assert!(
        thin_triangle(
            Vector3::new(0.25, -0.1, 0.25),
            Vector3::new(0.0, 0.2, 0.0),
            vertices()
        )
        .is_none()
    );
    assert!(
        thin_triangle(
            Vector3::new(0.75, 0.1, 0.75),
            Vector3::new(0.0, -0.2, 0.0),
            vertices()
        )
        .is_none()
    );
}

#[test]
fn native_tolerance_accepts_small_endpoint_overrun_without_clamping() {
    let start = Vector3::new(0.25, 0.200001, 0.25);
    let hit = thin_triangle(start, Vector3::new(0.0, -0.2, 0.0), vertices()).unwrap();
    assert!(hit.fraction > 1.0);
    assert!(
        thin_triangle(
            Vector3::new(0.25, 0.201, 0.25),
            Vector3::new(0.0, -0.2, 0.0),
            vertices()
        )
        .is_none()
    );
    assert!(thin_triangle(start, Vector3::ZERO, vertices()).is_none());
}

fn sweep(start: Vector3, delta: Vector3, radius: f32, fatness: f32) -> Option<TriangleLineHit> {
    let mut result = TriangleLineHit {
        position: Vector3::ZERO,
        normal: Vector3::ZERO,
        fraction: 0.0,
        volume_parameter: [0.0; 3],
    };
    triangle_segment(&mut result, start, delta, vertices(), radius, fatness).then_some(result)
}

#[test]
fn camera_sweep_resolves_face_edges_vertices_and_query_radius_offset() {
    let delta = Vector3::new(0.0, -2.0, 0.0);
    let face = sweep(Vector3::new(0.25, 1.0, 0.25), delta, 0.1, 0.0).unwrap();
    assert!((face.fraction - 0.45).abs() < 1e-6);
    assert!(face.position.y.abs() < 1e-6);
    let rounded_face = sweep(Vector3::new(0.25, 1.0, 0.25), delta, 0.1, 0.2).unwrap();
    assert!((rounded_face.fraction - 0.35).abs() < 1e-6);
    assert!((rounded_face.position.y - 0.2).abs() < 1e-6);
    let edge = sweep(Vector3::new(-0.06, 1.0, 0.5), delta, 0.1, 0.0).unwrap();
    assert!((edge.fraction - 0.46).abs() < 1e-5);
    assert!((edge.normal.x + 0.6).abs() < 1e-4);
    assert!((edge.normal.y - 0.8).abs() < 1e-4);
    assert!(edge.position.x.abs() < 1e-5 && edge.position.y.abs() < 1e-5);
    assert_eq!(edge.volume_parameter, [0.5, 0.0, 0.0]);
    let corner = sweep(Vector3::new(-0.04, 1.0, -0.03), delta, 0.1, 0.0).unwrap();
    let expected = (1.0 - (0.01_f32 - 0.0025).sqrt()) * 0.5;
    assert!((corner.fraction - expected).abs() < 1e-5);
    assert!(dot(corner.position, corner.position) < 1e-9);
    assert_eq!(corner.volume_parameter, [0.0; 3]);
    assert!(sweep(Vector3::new(-0.2, 1.0, 0.5), delta, 0.1, 0.0).is_none());
}

#[test]
fn rounded_query_handles_reverse_face_overlap_and_parallel_motion() {
    let reverse = sweep(
        Vector3::new(0.25, -1.0, 0.25),
        Vector3::new(0.0, 2.0, 0.0),
        0.1,
        0.0,
    )
    .unwrap();
    assert!((reverse.fraction - 0.45).abs() < 1e-6);
    assert!((reverse.normal.y + 1.0).abs() < 1e-6);
    let overlap = sweep(Vector3::new(0.25, 0.05, 0.25), Vector3::ZERO, 0.1, 0.0).unwrap();
    assert_eq!(overlap.fraction, 0.0);
    assert!((overlap.volume_parameter[2] - 0.0075).abs() < 1e-6);
    let parallel = sweep(
        Vector3::new(-1.0, 0.06, 0.5),
        Vector3::new(2.0, 0.0, 0.0),
        0.1,
        0.0,
    )
    .unwrap();
    assert!((parallel.fraction - 0.46).abs() < 1e-5);
    assert!((parallel.normal.x + 0.8).abs() < 1e-4);
    assert!((parallel.normal.y - 0.6).abs() < 1e-4);
}
