//! Immutable Skate 3 triangle preparation retained by actual Skate 2 arena
//! occurrence. Streaming still chooses every source and its order in Skate 2.
use crate::static_ground_pool::{ResidentGroundGeometry, ResidentGroundIdentity};
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{BoardWorld, PreparedTriangles, WorldTriangle},
        contact::RetailContactMaterial,
    },
};
use std::sync::Arc;

struct PreparedMesh {
    identity: ResidentGroundIdentity,
    triangles: Arc<PreparedTriangles>,
}

#[derive(Default)]
pub(crate) struct PreparedTerrain {
    meshes: Vec<PreparedMesh>,
    material_bits: Option<[u32; 3]>,
}

fn material_bits(material: RetailContactMaterial) -> [u32; 3] {
    [
        material.static_friction,
        material.dynamic_friction,
        material.restitution,
    ]
    .map(f32::to_bits)
}

impl PreparedTerrain {
    /// The initial Scene has already called the source geometry constructor.
    /// Retain those exact results instead of preparing the initial meshes twice.
    pub(crate) fn seed(
        sources: &[ResidentGroundGeometry],
        world: &BoardWorld,
        material: RetailContactMaterial,
    ) -> Result<Self, String> {
        if sources
            .iter()
            .map(|source| source.triangles.len())
            .sum::<usize>()
            != world.triangles().len()
        {
            return Err("Prepared terrain does not match the admitted world".into());
        }
        let mut offset = 0;
        let meshes = sources
            .iter()
            .map(|source| {
                let end = offset + source.triangles.len();
                let triangles = Arc::new(
                    world
                        .prepared_triangle_range(offset..end)
                        .expect("validated source partition covers the admitted world"),
                );
                offset = end;
                PreparedMesh {
                    identity: source.identity(),
                    triangles,
                }
            })
            .collect();
        Ok(Self {
            meshes,
            material_bits: Some(material_bits(material)),
        })
    }

    pub(crate) fn world(
        &mut self,
        sources: &[ResidentGroundGeometry],
        material: RetailContactMaterial,
    ) -> Result<BoardWorld, String> {
        let start = std::time::Instant::now();
        let count = sources
            .iter()
            .map(|source| source.triangles.len())
            .sum::<usize>();
        if count == 0 {
            return Err("Skate 3 player requires actual Skate 2 terrain".into());
        }
        let key = material_bits(material);
        let same_material = self.material_bits == Some(key);
        let mut meshes = Vec::with_capacity(sources.len());
        let mut reused = 0;
        for source in sources {
            if source.triangles.len() != source.features.len() {
                return Err("Skate 2 resident ground has incomplete native features".into());
            }
            let cached = same_material
                .then(|| {
                    self.meshes
                        .iter()
                        .find(|mesh| mesh.identity.matches(source))
                })
                .flatten();
            let triangles = if let Some(cached) = cached {
                reused += 1;
                Arc::clone(&cached.triangles)
            } else {
                let triangles = source
                    .triangles
                    .iter()
                    .zip(&source.features)
                    .map(|(triangle, feature)| {
                        if feature.edge_cosines.iter().any(|value| !value.is_finite()) {
                            return Err("Host collision contains a non-finite edge cosine");
                        }
                        WorldTriangle::from_vertices(
                            triangle.vertices.map(|[x, y, z]| Vector3::new(x, y, z)),
                            material,
                            u32::from(triangle.surface_id.unwrap_or(0)),
                            feature.flags,
                            feature.edge_cosines,
                            0.,
                        )
                        .ok_or("Invalid host native triangle")
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Arc::new(PreparedTriangles::new(triangles))
            };
            meshes.push(PreparedMesh {
                identity: source.identity(),
                triangles,
            });
        }
        let chunks: Vec<_> = meshes
            .iter()
            .map(|mesh| Arc::clone(&mesh.triangles))
            .collect();
        let prepared = PreparedTriangles::concatenate(&chunks);
        let packed_surfaces = prepared
            .triangles()
            .iter()
            .map(|triangle| triangle.tag as u16)
            .collect();
        let geometry_ms = start.elapsed().as_secs_f64() * 1000.;
        let world = crate::skate3_native::world_types::native_prepared_world(
            prepared,
            packed_surfaces,
            geometry_ms,
        )?;
        crate::diagnostics::test_log::event(
            "skate3_terrain_prepared",
            format_args!(
                "sources={} reused={reused} prepared={} triangles={count} geometry_ms={geometry_ms:.3}",
                meshes.len(),
                meshes.len() - reused
            ),
        );
        // Commit only after all geometry and canonical metadata pass validation.
        self.meshes = meshes;
        self.material_bits = Some(key);
        Ok(world)
    }
}

#[cfg(test)]
pub(crate) fn assert_original_world(
    sources: &[ResidentGroundGeometry],
    material: RetailContactMaterial,
    actual: &BoardWorld,
) {
    let expected = super::scene(sources, [0.; 3])
        .unwrap()
        .world(material)
        .unwrap();
    assert_eq!(actual.triangles().len(), expected.triangles().len());
    for (actual, expected) in actual.triangles().iter().zip(expected.triangles()) {
        fn bits(t: &WorldTriangle) -> ([u32; 31], u32, u32) {
            let t0 = &t.triangle;
            let vectors = [
                t0.vertices[0],
                t0.vertices[1],
                t0.vertices[2],
                t0.feature.normal,
                t0.feature.edges[0],
                t0.feature.edges[1],
                t0.feature.edges[2],
            ];
            let mut result = [0; 31];
            for (chunk, vector) in result[..21].chunks_exact_mut(3).zip(vectors) {
                chunk.copy_from_slice(&[vector.x, vector.y, vector.z].map(f32::to_bits));
            }
            result[21..24].copy_from_slice(&t0.edge_lengths.map(f32::to_bits));
            result[24..27].copy_from_slice(&t0.feature.edge_cosines.map(f32::to_bits));
            result[27..30].copy_from_slice(&material_bits(t.material));
            result[30] = t0.fatness.to_bits();
            (result, t0.feature.flags, t.tag)
        }
        assert_eq!(
            bits(actual),
            bits(expected),
            "prepared canonical geometry bits/order/material"
        );
    }
    assert_eq!(
        actual.triangle_bounds().len(),
        expected.triangle_bounds().len()
    );
    for (a, e) in actual
        .triangle_bounds()
        .iter()
        .zip(expected.triangle_bounds())
    {
        for (a, e) in [(a.min, e.min), (a.max, e.max)] {
            assert_eq!(
                [a.x, a.y, a.z].map(f32::to_bits),
                [e.x, e.y, e.z].map(f32::to_bits),
                "cached triangle bounds bits"
            );
        }
    }
    let a = actual.query_metadata().unwrap();
    let e = expected.query_metadata().unwrap();
    assert_eq!(a.packed_surfaces, e.packed_surfaces);
    assert_eq!(a.island_flags, e.island_flags);
    assert_eq!(a.static_edges.len(), e.static_edges.len());
    assert_eq!(a.meshes.len(), e.meshes.len());
    for (a, e) in a.meshes.iter().zip(&e.meshes) {
        assert_eq!(a.triangle_range, e.triangle_range);
        assert_eq!(a.geometry, e.geometry);
        assert_eq!(a.rejection_flags, e.rejection_flags);
        assert_eq!(a.matching_group, e.matching_group);
        assert_eq!(a.pool, e.pool);
        assert_eq!(a.local_to_world, e.local_to_world);
        assert_eq!(a.world_to_local, e.world_to_local);
        for (a, e) in [
            (a.local_bounds.min, e.local_bounds.min),
            (a.local_bounds.max, e.local_bounds.max),
        ] {
            assert_eq!(
                [a.x, a.y, a.z].map(f32::to_bits),
                [e.x, e.y, e.z].map(f32::to_bits)
            );
        }
    }
}
