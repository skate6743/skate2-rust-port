//! Collision calculation port; Skate 2 owns all world rendering.
use crate::skate3_native::math_host::*;
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{
            BoardWorld, WorldTriangle,
            query_metadata::{Bounds, QueryMesh, QueryMetadata, QueryPool},
        },
        collision::TriangleFeature,
        contact::RetailContactMaterial,
        drive_frames::RetailAffineTransform,
    },
};
use skate_data::skate_map::SkateMap;
use std::collections::HashMap;

pub(crate) fn validate_runtime(map: &SkateMap) -> Result<(), String> {
    let _span = info_span!("validate_map").entered();
    let archive = retail_archive(map)?;
    if map.geometry.collision.is_empty() && archive.is_none() {
        return Err("SKATE map has no collision geometry".into());
    }
    for triangle in map.geometry.collision.iter().filter(|_| archive.is_none()) {
        if let Some(edges) = triangle.native_edges {
            decode_native_edges(edges)?;
        }
    }
    if !map.doors.is_empty() {
        return Err(format!(
            "Map '{}' contains {} hinged doors. This imported game has no door body/controller adapter yet; refusing to drop their geometry or turn them into static walls.",
            map.name,
            map.doors.len()
        ));
    }
    for extension in &map.extensions {
        let tag = String::from_utf8_lossy(&extension.tag);
        if extension.tag == *b"RWCM" {
            continue;
        }
        if extension.tag == skate_data::trigger_volumes::EXTENSION_TAG {
            // Named trigger volumes (data only, never collision); see trigger_volumes.rs.
            if extension.schema != skate_data::trigger_volumes::EXTENSION_SCHEMA {
                return Err(format!("SKATE extension TVOL schema {} is not supported", extension.schema));
            }
            skate_data::trigger_volumes::parse(&extension.payload)?;
            continue;
        }
        if extension.tag == *b"MOBJ" {
            skate_data::skate_map::validate_static_objects(map, extension)?;
            continue;
        }
        if extension.tag == *b"SKYB" && extension.schema == 1 {
            eprintln!(
                "SKATE LIMITATION: SKYB retail sky retained; using the map horizon until its shader adapter is available."
            );
            continue;
        }
        if extension.tag != *b"WMET" && extension.tag != *b"WCFG" && extension.tag != *b"BMAT" {
            return Err(format!(
                "SKATE extension {tag} schema {} is decoded but its runtime adapter is not implemented. Refusing to silently omit potentially required world geometry.",
                extension.schema
            ));
        }
        eprintln!(
            "SKATE LIMITATION: {tag} extension retained; its runtime behavior is not connected."
        );
    }
    if map.textures.iter().any(|t| t.width == 0) {
        return Err(
            "SKATE contains external texture placeholders; supply a package with embedded textures"
                .into(),
        );
    }
    if !map.routes.is_empty() {
        eprintln!(
            "SKATE LIMITATION: {} NPC routes parsed; supplied game has no NPC controller.",
            map.routes.len()
        );
    }
    if map.lights.iter().any(|l| l.kind == 2) {
        eprintln!(
            "SKATE LIMITATION: area-light records retained; Bevy adapter currently renders point and spot lights only."
        );
    }
    eprintln!(
        "SKATE_MAP_LOADED name={:?} version={} render_triangles={} collision_triangles={} textures={} spawn={:?}",
        map.name,
        map.version,
        map.geometry.indices.len() / 3,
        map.geometry.collision.len(),
        map.textures.len(),
        map.spawn
    );
    Ok(())
}

/// TU3 ClusteredMesh::GetUnitVolumes (82AC8A68): fdivs then fsubs,
/// using the pi-squared word at 822F88D0. This is not acos/angle decoding.
/// Bit 7 denotes an unmatched compiler edge and is not a triangle flag.
fn decode_native_edges(edges: [u8; 3]) -> Result<(u32, [f32; 3]), String> {
    let mut flags = 1 | TriangleFeature::ONE_SIDED | TriangleFeature::USE_EDGE_COSINES;
    let mut cosines = [0.; 3];
    for (i, code) in edges.into_iter().enumerate() {
        let exponent = code & 0x1f;
        // The native signed 32-bit shift becomes negative at 28 and zero
        // above it. Reject those malformed codes instead of producing NaNs.
        if exponent >= 28 {
            return Err(format!(
                "Invalid SKATE native edge angle code {exponent} at corner {i}"
            ));
        }
        cosines[i] = 1.0 - f32::from_bits(0x411d_e9e7) / ((8_u32 << exponent) as f32);
        flags |= u32::from(code & 0x20) << i;
        flags |= u32::from(code & 0x40) << (i + 3);
    }
    Ok((flags, cosines))
}

fn retail_archive(map: &SkateMap) -> Result<Option<&[u8]>, String> {
    let mut archives = map.extensions.iter().filter(|e| e.tag == *b"RWCM");
    let Some(archive) = archives.next() else {
        return Ok(None);
    };
    if archive.schema != 1 || archives.next().is_some() {
        return Err("SKATE requires one RWCM extension with schema 1".into());
    }
    Ok(Some(&archive.payload))
}

fn retail_collision_world(
    archive: &[u8],
    material: RetailContactMaterial,
) -> Result<BoardWorld, String> {
    let mut triangles = Vec::new();
    let mut packed_surfaces = Vec::new();
    let mut meshes = Vec::new();
    let count = skate_data::retail_collision::visit_clusters(archive, |_, cluster| {
        // Preserve cluster order and partition further only for group filters.
        let mut cursor = 0;
        while cursor < cluster.len() {
            let group = cluster[cursor].group;
            let start = triangles.len();
            while cursor < cluster.len() && cluster[cursor].group == group {
                let source = cluster[cursor];
                let (flags, cosines) = match source.edges {
                    Some(edges) => {
                        let (mut flags, cosines) = decode_native_edges(edges)?;
                        flags &= !TriangleFeature::ONE_SIDED;
                        if source.one_sided {
                            flags |= TriangleFeature::ONE_SIDED;
                        }
                        (flags, cosines)
                    }
                    // TriangleVolume ctor82AC7770 retains these defaults when
                    // the unit has no edge data; the mesh sidedness is not read.
                    None => (0x1e1, [-1.; 3]),
                };
                triangles.push(
                    WorldTriangle::from_vertices(
                        source.points.map(|p| Vector3::new(p[0], p[1], p[2])),
                        material,
                        u32::from(source.surface),
                        flags,
                        cosines,
                        0.,
                    )
                    .ok_or("Invalid RWCM collision triangle")?,
                );
                packed_surfaces.push(source.surface);
                cursor += 1;
            }
            let range = start..triangles.len();
            let bounds = Bounds::from_points(
                triangles[range.clone()]
                    .iter()
                    .flat_map(|t| t.triangle.vertices),
            )
            .ok_or("Invalid RWCM cluster bounds")?;
            meshes.push(QueryMesh {
                geometry: 0, rejection_flags: 0,
                triangle_range: range,
                local_to_world: RetailAffineTransform::IDENTITY,
                world_to_local: RetailAffineTransform::IDENTITY,
                local_bounds: bounds,
                // Native static registration uses matchingID -1; the unit group
                // only splits clusters and must not filter actor queries.
                matching_group: -1,
                pool: QueryPool::Ground,
            });
        }
        Ok(())
    })?;
    eprintln!(
        "SKATE_RWCM_READY triangles={count} query_clusters={} source=embedded",
        meshes.len()
    );
    BoardWorld::with_query_metadata(
        triangles,
        QueryMetadata {
            packed_surfaces,
            meshes,
            static_edges: vec![],
            island_flags: 0,
        },
    )
    .map_err(str::to_owned)
}

pub(crate) fn collision_world(
    map: &SkateMap,
    material: RetailContactMaterial,
) -> Result<BoardWorld, String> {
    if let Some(archive) = retail_archive(map)? {
        return retail_collision_world(archive, material);
    }
    // Match the reference RW mesh compiler's 1 mm vertex welding and reversed
    // edge pairing. Triangle diagonals are adjacency, never authored ledges.
    let mut welded = HashMap::<[i64; 3], usize>::new();
    let mut positions = Vec::<Vec3>::new();
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    for tri in &map.geometry.collision {
        let ids = tri.points.map(|p| {
            let inverse = 1.0 / f64::from(0.001_f32);
            let key = p.map(|v| (f64::from(v) * inverse).round() as i64);
            *welded.entry(key).or_insert_with(|| {
                let id = positions.len();
                positions.push(Vec3::from_array(p));
                id
            })
        });
        let [a, b, c] = tri.points.map(Vec3::from_array);
        let normal = (b - a)
            .cross(c - a)
            .try_normalize()
            .ok_or("Invalid SKATE collision triangle normal")?;
        vertices.push(ids);
        normals.push(normal);
    }
    let mut cosines = vec![[1.; 3]; vertices.len()];
    let mut flags =
        vec![TriangleFeature::ONE_SIDED | TriangleFeature::USE_EDGE_COSINES | 0xe0; vertices.len()];
    // Fully native maps need no reconstructed adjacency. Mixed maps still
    // include every face when finding neighbors for their authored geometry.
    if map
        .geometry
        .collision
        .iter()
        .any(|t| t.native_edges.is_none())
    {
        let mut open = HashMap::<(usize, usize), (usize, usize)>::new();
        for (i, ids) in vertices.iter().enumerate() {
            for edge in 0..3 {
                let (a, b) = (ids[edge], ids[(edge + 1) % 3]);
                if let Some((other, oe)) = open.remove(&(b, a)) {
                    let cosine = normals[i].dot(normals[other]).clamp(-1., 1.);
                    let orientation =
                        (positions[b] - positions[a]).dot(normals[i].cross(normals[other]));
                    // ExtendedEdgeCosine / MakeEdgeCode in rw_collision_mesh.cpp:
                    // orientation >= -1e-6 is convex; flat edges have no convex bit.
                    for (ti, e) in [(i, edge), (other, oe)] {
                        cosines[ti][e] = cosine;
                        if orientation <= -1.0e-6 || cosine >= 1. {
                            flags[ti] &= !(0x20 << e);
                        }
                    }
                } else {
                    open.entry((a, b)).or_insert((i, edge));
                }
            }
        }
        let mut adjacent = vec![Vec::new(); positions.len()];
        for (i, ids) in vertices.iter().enumerate() {
            for &v in ids {
                adjacent[v].push(i);
            }
        }
        for (v, faces) in adjacent.iter().enumerate() {
            let reference = normals[faces[0]];
            if faces
                .iter()
                .all(|&i| (reference.dot(normals[i]) - 1.).abs() <= 0.01)
            {
                for &i in faces {
                    for corner in 0..3 {
                        if vertices[i][corner] == v {
                            flags[i] |= 0x200 << corner;
                        }
                    }
                }
            }
        }
    }
    let mut triangles = Vec::with_capacity(vertices.len());
    let mut packed_surfaces = Vec::with_capacity(vertices.len());
    for (i, source) in map.geometry.collision.iter().enumerate() {
        if let Some(edges) = source.native_edges {
            (flags[i], cosines[i]) = decode_native_edges(edges)?;
        }
        let m = &map.materials[source.material as usize - 1];
        // Exact EncodeRwSurfaceId mapping from the reference native adapter.
        packed_surfaces.push((m.audio | (m.physics << 7) | (m.pattern << 12)) as u16);
        let points = vertices[i].map(|id| {
            let p = positions[id];
            Vector3::new(p.x, p.y, p.z)
        });
        // Keep the supplied game's original static-world contact combine values.
        // The native map bridge supplies packed surfaces, not a guessed split of
        // the package's single friction scalar into static/dynamic coefficients.
        triangles.push(
            WorldTriangle::from_vertices(
                points,
                material,
                source.surface,
                flags[i],
                cosines[i],
                0.,
            )
            .ok_or("Invalid SKATE collision volume")?,
        );
    }
    // Portable maps have no native cluster hierarchy. Bound contiguous ranges
    // once at load time so the existing BVH can reject distant geometry. Keep
    // triangle order and mesh identity/filter values: contact tie-breaking,
    // packed surfaces and adjacency must not change with this acceleration.
    let mut meshes = Vec::new();
    for start in (0..triangles.len()).step_by(64) {
        let end = (start + 64).min(triangles.len());
        let bounds = Bounds::from_points(triangles[start..end].iter().flat_map(|t| t.triangle.vertices))
            .ok_or("SKATE collision bounds empty")?;
        meshes.push(QueryMesh {
            geometry: 0, rejection_flags: 0,
            triangle_range: start..end,
            local_to_world: RetailAffineTransform::IDENTITY,
            world_to_local: RetailAffineTransform::IDENTITY,
            local_bounds: bounds,
            matching_group: -1,
            pool: QueryPool::Ground,
        });
    }
    let metadata = QueryMetadata {
        packed_surfaces,
        meshes,
        static_edges: vec![],
        island_flags: 0,
    };
    BoardWorld::with_query_metadata(triangles, metadata).map_err(str::to_owned)
}

#[cfg(any())]
mod tests {
    use super::*;

    /// Measures the static draw budget on a real installed map. This is the
    /// check that the whole architecture exists to pass, so it reports the class
    /// mix rather than only the total: if the budget is ever missed, the mix says
    /// whether to subdivide less or to stop splitting on a material property.
    ///
    /// `SKATE_BUDGET_MAP` names the `.skate` file; `SKATE_TRANSITION_TEST_ASSETS`
    /// gives the asset root.
    #[test]
    #[ignore = "Requires private installed assets; CPU only, no GPU or window"]
    fn static_draw_budget_holds_on_an_installed_map() {
        let root = std::path::PathBuf::from(
            std::env::var("SKATE_TRANSITION_TEST_ASSETS")
                .expect("SKATE_TRANSITION_TEST_ASSETS"),
        );
        let path = std::path::PathBuf::from(
            std::env::var("SKATE_BUDGET_MAP").expect("SKATE_BUDGET_MAP"),
        );
        let map = skate_data::skate_map::SkateMap::load(&path).unwrap();
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<Assets<crate::skate3_native::retail_render::WorldMaterial>>();
        world.init_resource::<Assets<bevy::render::storage::ShaderStorageBuffer>>();
        let mut scene = crate::skate3_native::map_render::PreparedScene::new(&world);
        let started = std::time::Instant::now();
        scene.prepare(Some(&map), &root);
        let elapsed = started.elapsed();
        let stats = scene.stats;
        eprintln!(
            "SKATE_BUDGET map={} draws={} triangles={} slabs={} materials={} prepare_ms={}",
            path.file_stem().unwrap_or_default().to_string_lossy(),
            stats.draws,
            stats.triangles,
            stats.slabs,
            map.materials.len(),
            elapsed.as_millis()
        );
        assert!(
            stats.draws < 300,
            "static draw budget is under 300; got {} draws",
            stats.draws
        );
        assert!(stats.triangles > 0, "map produced no geometry");
    }

    fn triangle(centroid: Vec3) -> Triangle {
        Triangle { indices: [0, 1, 2], centroid, slab: 0, class: RenderClass::Opaque }
    }

    #[test]
    fn partition_respects_the_leaf_budget() {
        let triangles: Vec<Triangle> = (0..LEAF_TRIANGLE_BUDGET * 3 + 7)
            .map(|i| triangle(Vec3::new(i as f32, 0., 0.)))
            .collect();
        let mut members: Vec<usize> = (0..triangles.len()).collect();
        let leaves = partition(&mut members, &triangles);
        assert!(leaves.len() >= 4, "expected several leaves, got {}", leaves.len());
        for leaf in &leaves {
            assert!(leaf.len() <= LEAF_TRIANGLE_BUDGET, "leaf of {} triangles", leaf.len());
        }
    }

    #[test]
    fn partition_preserves_every_triangle_exactly_once() {
        let triangles: Vec<Triangle> = (0..5_000)
            .map(|i| triangle(Vec3::new((i % 71) as f32, (i % 13) as f32, i as f32)))
            .collect();
        let mut members: Vec<usize> = (0..triangles.len()).collect();
        let mut seen: Vec<usize> = partition(&mut members, &triangles)
            .into_iter()
            .flatten()
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..triangles.len()).collect::<Vec<_>>());
    }

    #[test]
    fn a_single_leaf_is_not_split() {
        let triangles: Vec<Triangle> = (0..64).map(|i| triangle(Vec3::splat(i as f32))).collect();
        let mut members: Vec<usize> = (0..triangles.len()).collect();
        assert_eq!(partition(&mut members, &triangles).len(), 1);
    }
}
