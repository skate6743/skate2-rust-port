use super::*;

#[test]
fn flat_render_vertices_and_actual_queries_agree_across_entire_surface() {
    let surfaces = Terrain::Flat.surfaces();
    assert_eq!(surfaces.iter().map(Vec::len).sum::<usize>(), 1);
    for vertex in surfaces.iter().flatten().flatten() {
        assert_eq!(vertex.y, HEIGHT);
    }
    // Test-only material: geometry checks do not exercise contact response.
    let world = Terrain::Flat.world(RetailContactMaterial {
        static_friction: 0.0,
        dynamic_friction: 0.0,
        restitution: 0.0,
    });
    assert_eq!(world.triangles().len(), 2);
    let metadata = world.query_metadata().unwrap();
    assert!(metadata.static_edges.is_empty());
    assert_eq!(metadata.packed_surfaces.len(), 2);
    for entry in world.triangles() {
        for vertex in entry.triangle.vertices {
            assert_eq!(vertex.y, HEIGHT);
        }
    }
    // Includes former downhill route, half-pipe, seam and near each boundary.
    for x in [-49.9, -12.0, -3.0, 0.0, 3.0, 12.0, 49.9] {
        for z in [-49.9, -16.0, -4.0, 0.0, 4.0, 8.0, 16.0, 49.9] {
            let hit = world
                .query_thin_line(Vector3::new(x, 2.0, z), Vector3::new(x, -2.0, z))
                .unwrap()
                .expect("flat surface query");
            assert!((hit.geometry.position.y - HEIGHT).abs() < 1.0e-5);
            assert!(hit.geometry.normal.x.abs() < 1.0e-6);
            assert!(hit.geometry.normal.z.abs() < 1.0e-6);
            assert!((hit.geometry.normal.y - 1.0).abs() < 1.0e-6);
        }
    }
}
