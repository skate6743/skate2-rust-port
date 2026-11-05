//! Host boundary for the original production tick, with caller-owned scene data.


use skate3_core::{
    input::xbox::XboxState,
    physics::{
        board::BodyId,
        skeleton_animation_record::{AnimationPartTransform, compose_affine},
    },
};
use skate_data::{
    GameAssets,
    skate_map::{Collision, Geometry, Material, Rail, SkateMap},
};


#[derive(Clone, Copy, Debug, Default)]
pub struct RawController {
    pub buttons: u16,
    pub triggers: [u8; 2],
    pub left: [i16; 2],
    pub right: [i16; 2],
}

#[derive(Clone, Debug)]
pub struct SceneTriangle {
    pub points: [[f32; 3]; 3],
    pub surface: u32,
    /// RenderWare's packed audio (7), physics (5), and pattern (4) bits.
    pub packed_surface: u16,
    pub native_edges: Option<[u8; 3]>,
    pub native_features: Option<SceneTriangleFeatures>,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneTriangleFeatures {
    pub flags: u32,
    pub edge_cosines: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct SceneRail {
    pub name: String,
    pub points: Vec<[f32; 3]>,
    pub closed: bool,
    pub native: Option<Vec<u8>>,
}

/// Native host spline data with occurrence-qualified header handles. The
/// source runtime computes its own contact chord and loose-octree traversal.
#[derive(Clone, Debug)]
pub struct HostGrindSegment {
    pub coefficients: [[f32; 4]; 4],
    pub minimum: [f32; 3],
    pub maximum: [f32; 3],
    pub owner: u64,
    pub spline_guids: [u64; 2],
    pub segment_index: u32,
}
#[derive(Clone, Debug)]
pub struct HostGrindAsset {
    pub segments: Vec<HostGrindSegment>,
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub spawn: [f32; 3],
    pub heading: f32,
    pub triangles: Vec<SceneTriangle>,
    pub rails: Vec<SceneRail>,
}

impl Scene {
    /// Already-decoded native RenderWare features are retained verbatim. An
    /// all-authored scene continues using the local engine's map compiler.
    pub(crate) fn world(
        &self,
        material: skate3_core::physics::contact::RetailContactMaterial,
    ) -> Result<skate3_core::physics::board_world::BoardWorld, String> {
        use skate3_core::{
            math::Vector3,
            physics::{
                board_world::WorldTriangle,
            },
        };
        if self
            .triangles
            .iter()
            .all(|triangle| triangle.native_features.is_none())
        {
            return crate::skate3_native::skate_world::collision_world(&self.map()?, material);
        }
        let geometry_start = std::time::Instant::now();
        let mut triangles = Vec::with_capacity(self.triangles.len());
        for source in &self.triangles {
            let feature = source
                .native_features
                .ok_or("Host collision must supply native features for every triangle")?;
            if feature.edge_cosines.iter().any(|v| !v.is_finite()) {
                return Err("Host collision contains a non-finite edge cosine".into());
            }
            let points = source.points.map(|[x, y, z]| Vector3::new(x, y, z));
            triangles.push(
                WorldTriangle::from_vertices(
                    points,
                    material,
                    source.surface,
                    feature.flags,
                    feature.edge_cosines,
                    0.,
                )
                .ok_or("Invalid host native triangle")?,
            );
        }
        let geometry_ms = geometry_start.elapsed().as_secs_f64() * 1000.;
        native_world(triangles,
            self.triangles.iter().map(|triangle|triangle.packed_surface).collect(),geometry_ms)
    }

    pub(crate) fn map(&self) -> Result<SkateMap, String> {
        if self.triangles.is_empty() {
            return Err("Embedded Skate 3 requires the host's real collision scene".into());
        }
        let mut surface_ids = std::collections::BTreeMap::new();
        let mut materials = Vec::new();
        let mut collision = Vec::with_capacity(self.triangles.len());
        for triangle in &self.triangles {
            let material = *surface_ids
                .entry(triangle.packed_surface)
                .or_insert_with(|| {
                    let packed = u32::from(triangle.packed_surface);
                    // This map is collision-only. The source collision adapter reads
                    // these three surface fields; contact coefficients still come
                    // from the original Skate 3 settings, never these visual fields.
                    materials.push(Material {
                        name: format!("host-surface-{packed:04x}"),
                        flags: 0,
                        friction: 0.,
                        restitution: 0.,
                        color: [0.; 3],
                        roughness: 0.,
                        emissive: 0.,
                        textures: [0; 5],
                        indirect_strength: 0.,
                        alpha_mode: 0,
                        alpha_cutoff: 0.,
                        audio: packed & 0x7f,
                        physics: (packed >> 7) & 0x1f,
                        pattern: packed >> 12,
                        depth_layer: None,
                        retail_definition: None,
                    });
                    materials.len() as u32
                });
            collision.push(Collision {
                points: triangle.points,
                surface: triangle.surface,
                material,
                native_edges: triangle.native_edges,
            });
        }
        Ok(SkateMap {
            version: 15,
            name: "Skate 2 resident world".into(),
            spawn: self.spawn,
            heading: self.heading,
            environment: Vec::new(),
            materials,
            textures: Vec::new(),
            geometry: Geometry {
                vertices: Vec::new(),
                indices: Vec::new(),
                collision,
            },
            rails: self
                .rails
                .iter()
                .map(|rail| Rail {
                    name: rail.name.clone(),
                    points: rail.points.clone(),
                    closed: rail.closed,
                    native: rail.native.clone(),
                })
                .collect(),
            doors: Vec::new(),
            lights: Vec::new(),
            routes: Vec::new(),
            extensions: Vec::new(),
        })
    }
}

/// Assemble already prepared canonical native triangles in their supplied order.
/// Leaf boundaries remain the original global 64-triangle partition, including
/// boundaries spanning source meshes. Ordinary validation still runs in full.
fn native_query_metadata(
    triangles: &[skate3_core::physics::board_world::WorldTriangle],
    packed_surfaces: Vec<u16>,
    prepared_bounds: Option<&[skate3_core::physics::board_world::query_metadata::Bounds]>,
) -> Result<skate3_core::physics::board_world::query_metadata::QueryMetadata, String> {
    use skate3_core::math::Vector3;
    use skate3_core::physics::{
        board_world::query_metadata::{Bounds, QueryMesh, QueryMetadata, QueryPool},
        drive_frames::RetailAffineTransform,
    };
    let mut meshes = Vec::new();
    for start in (0..triangles.len()).step_by(64) {
        let end = (start + 64).min(triangles.len());
        let mut cached = prepared_bounds.map(|bounds| {
            let mut combined = bounds[start];
            for bounds in &bounds[start + 1..end] {
                combined.min = Vector3::new(
                    combined.min.x.min(bounds.min.x),
                    combined.min.y.min(bounds.min.y),
                    combined.min.z.min(bounds.min.z),
                );
                combined.max = Vector3::new(
                    combined.max.x.max(bounds.max.x),
                    combined.max.y.max(bounds.max.y),
                    combined.max.z.max(bounds.max.z),
                );
            }
            combined
        });
        // f32 min/max may choose either signed zero on a tie. Preserve the
        // original vertex order for that case and for invalid-point sentinels.
        if cached.is_some_and(|bounds| {
            [
                bounds.min.x,
                bounds.min.y,
                bounds.min.z,
                bounds.max.x,
                bounds.max.y,
                bounds.max.z,
            ]
            .into_iter()
            .any(|value| value == 0. || !value.is_finite())
        }) {
            cached = None;
        }
        let local_bounds = cached
            .or_else(|| {
                Bounds::from_points(
                    triangles[start..end]
                        .iter()
                        .flat_map(|triangle| triangle.triangle.vertices),
                )
            })
            .ok_or("Empty host collision bounds")?;
        meshes.push(QueryMesh {
            geometry: 0,
            rejection_flags: 0,
            triangle_range: start..end,
            local_to_world: RetailAffineTransform::IDENTITY,
            world_to_local: RetailAffineTransform::IDENTITY,
            local_bounds,
            matching_group: -1,
            pool: QueryPool::Ground,
        });
    }
    Ok(QueryMetadata {
        packed_surfaces,
        meshes,
        static_edges: Vec::new(),
        island_flags: 0,
    })
}

fn log_native_world(
    world: &skate3_core::physics::board_world::BoardWorld,
    timing: skate3_core::physics::board_world::WorldConstructionTimings,
    geometry_ms: f64,
    metadata_ms: f64,
) {
    crate::diagnostics::test_log::event(
        "skate3_world_build",
        format_args!(
            "triangles={} geometry_ms={geometry_ms:.3} metadata_ms={metadata_ms:.3} validation_ms={:.3} triangle_data_ms={:.3} query_index_ms={:.3}",
            world.triangles().len(),
            timing.validation.as_secs_f64() * 1000.,
            timing.triangle_data.as_secs_f64() * 1000.,
            timing.query_index.as_secs_f64() * 1000.
        ),
    );
}

pub(crate) fn native_world(
    triangles: Vec<skate3_core::physics::board_world::WorldTriangle>,
    packed_surfaces: Vec<u16>,
    geometry_ms: f64,
) -> Result<skate3_core::physics::board_world::BoardWorld, String> {
    let start = std::time::Instant::now();
    let metadata = native_query_metadata(&triangles, packed_surfaces, None)?;
    let metadata_ms = start.elapsed().as_secs_f64() * 1000.;
    let (world, timing) =
        skate3_core::physics::board_world::BoardWorld::with_query_metadata_profiled(
            triangles, metadata,
        )
        .map_err(str::to_owned)?;
    log_native_world(&world, timing, geometry_ms, metadata_ms);
    Ok(world)
}

pub(crate) fn native_prepared_world(
    prepared: skate3_core::physics::board_world::PreparedTriangles,
    packed_surfaces: Vec<u16>,
    geometry_ms: f64,
) -> Result<skate3_core::physics::board_world::BoardWorld, String> {
    let start = std::time::Instant::now();
    let metadata = native_query_metadata(
        prepared.triangles(),
        packed_surfaces,
        Some(prepared.triangle_bounds()),
    )?;
    let metadata_ms = start.elapsed().as_secs_f64() * 1000.;
    let (world, timing) =
        skate3_core::physics::board_world::BoardWorld::with_prepared_query_metadata_profiled(
            prepared, metadata,
        )
        .map_err(str::to_owned)?;
    log_native_world(&world, timing, geometry_ms, metadata_ms);
    Ok(world)
}

#[derive(Clone, Copy, Debug)]
pub struct BodyOutput {
    pub transform: AnimationPartTransform,
    pub centre_of_mass: [f32; 3],
    pub velocity: [f32; 3],
    pub omega: [f32; 3],
}

#[derive(Clone, Debug)]
pub struct SkeletonPartOutput {
    /// Original physics-to-animation name binding, not a host part index.
    pub bone_name: String,
    pub body: BodyOutput,
    pub bone_transform: AnimationPartTransform,
}

#[derive(Clone, Debug)]
pub struct FrameOutput {
    /// Completed reset callback, including automatic bail recovery.
    pub teleported: bool,
    pub gameplay: GameplayOutput,
    pub tick: u64,
    pub physical: skate3_core::player::input_phase::PhysicalPlayerInput,
    /// Completed world-space publication, distinct from the animation record's
    /// local-space centre of mass.
    pub centre_of_mass: [f32; 4],
    pub damped_centre_of_mass: skate3_core::physics::centre_of_mass_filter::CentreOfMassOutput,
    pub state: u32,
    pub category: u32,
    pub offboard: bool,
    pub wiping_out: bool,
    /// PhysOut_State mStateUpdateCount (State4): ticks in the current physical state.
    pub state_frames: u32,
    /// PhysOut_State mFramesSinceReset (State20): the physics conditioner's samples since its
    /// last reset (EmpiricalMeasurements82DE5858).
    pub frames_since_reset: i32,
    pub board_transform: AnimationPartTransform,
    /// Processed2468 bit20: GetEffectiveTransform82C01BF8 negates the deck's X and Z.
    pub board_flipped: bool,
    pub board_velocity: [f32; 3],
    pub board_parts: [BodyOutput; 7],
    pub skeleton_parts: Vec<SkeletonPartOutput>,
    pub root_velocity: [f32; 3],
    pub root_omega: [f32; 3],
    pub camera: Option<skate3_core::camera::CameraFrame>,
    pub root_transform: AnimationPartTransform,
    pub bone_names: Vec<String>,
    pub world_bones: Vec<AnimationPartTransform>,
    /// The pumping state Update5 publishes (TU3 82DB6FD0).
    pub pumping: skate3_core::riding::pumping::state::PumpingState,
}

impl FrameOutput {
    /// World facing used by Actor queries and markers. Native processed-input publication
    /// unmirrors animation Ri/At; the PC biped root already owns that mapping.
    pub fn effective_root_transform(&self) -> AnimationPartTransform {
        let mut root = self.root_transform;
        if !self.offboard && self.gameplay.stance[1] {
            for axis in [0, 2] { root[axis] = root[axis].map(|v| -v); }
        }
        root
    }
}

/// Completed source facts consumed by the host game's audio, scoring and career.
/// Contact geometry and forces are copied from the solved source owners.
#[derive(Clone, Debug)]
pub struct GameplayOutput {
    pub state_flags: [bool; 36], // Native state bytes 52..88.
    /// Processed object2464 and its actual PostInput2480 bit20 publication.
    pub interactable_object: (u32,bool),
    pub animation_turn: f32,
    pub landing_quality: skate3_core::animation::landing_quality::Output,
    pub jump_strength: f32,
    /// Completed TU3 skeleton/board trajectory facts used by air-gap and context scoring.
    pub hips_ground_start: [f32; 4],
    pub hips_ground: skate3_core::player::input_phase::LineTestFields,
    pub deck_ground: skate3_core::physics::board_probes::BoardProbeState,
    pub foot_surfaces: [u32; 2], // Left, right, packed audio/physics/pattern.
    pub deck_tilt: f32,
    pub wheel_hardness: f32,
    pub trick: Option<skate3_core::animation::output::attributes::AttributeName>,
    pub trick_flags: u32,
    pub trick_tweak: [f32; 2],
    pub wipeout_gesture: [f32; 2],
    pub stance: [bool; 4], // fakie, mirrored, switch, regular
    /// PhysOut_Animation mPushMongo/mPushRegular, set on a stroke's first pushing tick.
    pub push_style: [bool; 2],
    pub board_ground: skate3_core::physics::board_ground::BoardGroundState,
    pub wheel_lines: skate3_core::physics::board_ground::WheelLineState,
    pub bone_contacts: [skate3_core::physics::skeleton_body::BoneContact; 24],
    /// Pose-derived per-part velocity changes, as used by bone-impact scoring.
    pub bone_velocity_changes: [[f32; 4]; 24],
    pub contact_regions: [skate3_core::physics::skeleton_body::ContactRegion; 8],
    pub contact_flags: skate3_core::physics::skeleton_body::SkeletonContactFlags,
    pub specific_contacts: [skate3_core::physics::skeleton_body::SpecificContact; 2],
    pub wipeout_times: [f32; 3],
    pub entity_impacts: [f32; 3], // vehicle(group8), skater, pedestrian(group11)
    pub feet: skate3_core::physics::foot_physical_output::FootPhysicalOutput,
    /// This frame's heavy landing (Wipeout request 6) as TU3 Update5 82DB71C8 publishes it:
    /// kind 1 left footplant, 2 right footplant, 0 both feet; and the landing speed.
    pub heavy_landing: Option<(i32, f32)>,
}

/// Corrections produced by the source solver for a host-owned moving body.
#[derive(Clone, Copy, Debug)]
pub struct BodyReaction {
    pub id: u64,
    pub linear_velocity_delta: [f32; 3],
    pub angular_velocity_delta: [f32; 3],
    pub position_delta: [f32; 3],
    pub orientation_delta: [f32; 3],
}


#[cfg(test)]
mod host_surface_tests {
    use super::*;
    #[test]
    fn prepared_world_preserves_global_leaves_zero_bits_and_rounded_queries() {
        use skate3_core::{
            math::Vector3,
            physics::{
                board_world::{BoardWorld, PreparedTriangles, WorldTriangle},
                contact::RetailContactMaterial,
            },
        };
        use std::sync::Arc;
        let material = RetailContactMaterial {
            static_friction: 0.6,
            dynamic_friction: 0.4,
            restitution: 0.,
        };
        let bounds_bits = |b: &skate3_core::physics::board_world::query_metadata::Bounds| {
            [b.min.x, b.min.y, b.min.z, b.max.x, b.max.y, b.max.z].map(f32::to_bits)
        };
        let compare = |original: &BoardWorld, prepared: &BoardWorld| {
            assert_eq!(original.triangles().len(), prepared.triangles().len());
            assert_eq!(
                original
                    .triangle_bounds()
                    .iter()
                    .map(bounds_bits)
                    .collect::<Vec<_>>(),
                prepared
                    .triangle_bounds()
                    .iter()
                    .map(bounds_bits)
                    .collect::<Vec<_>>()
            );
            let a = original.query_metadata().unwrap();
            let b = prepared.query_metadata().unwrap();
            assert_eq!(a.packed_surfaces, b.packed_surfaces);
            assert_eq!(a.meshes.len(), b.meshes.len());
            for (a, b) in a.meshes.iter().zip(&b.meshes) {
                assert_eq!(a.triangle_range, b.triangle_range);
                assert_eq!(bounds_bits(&a.local_bounds), bounds_bits(&b.local_bounds));
            }
            for index in 0..130 {
                let x = 2. + index as f32 * 5.;
                for radius in [0., 0.2] {
                    assert_eq!(
                        original.query_swept_line(
                            Vector3::new(x, 12., 3.),
                            Vector3::new(x, -2., 3.),
                            radius
                        ),
                        prepared.query_swept_line(
                            Vector3::new(x, 12., 3.),
                            Vector3::new(x, -2., 3.),
                            radius
                        )
                    );
                }
            }
        };
        for zero_ties in [false, true] {
            let triangles: Vec<_> = (0..130)
                .map(|index| {
                    let x = 1. + index as f32 * 5.;
                    let zero = zero_ties && index % 7 == 0;
                    WorldTriangle::from_vertices(
                        [
                            Vector3::new(x, if zero { -0. } else { 10. }, 2.),
                            Vector3::new(x, if zero { 0. } else { 10. }, 6.),
                            Vector3::new(x + 4., if zero { -0. } else { 10. }, 2.),
                        ],
                        material,
                        0x3084 + index,
                        0x1e1,
                        [-1.; 3],
                        if index % 3 == 0 { 0.125 } else { 0. },
                    )
                    .unwrap()
                })
                .collect();
            let surfaces = triangles.iter().map(|t| t.tag as u16).collect::<Vec<_>>();
            let original = native_world(triangles.clone(), surfaces.clone(), 0.).unwrap();
            let chunks: Vec<_> = [0..17, 17..80, 80..130]
                .into_iter()
                .map(|range| Arc::new(PreparedTriangles::new(triangles[range].to_vec())))
                .collect();
            let prepared = native_prepared_world(
                PreparedTriangles::concatenate(&chunks),
                surfaces.clone(),
                0.,
            )
            .unwrap();
            compare(&original, &prepared);
            // Seed from a completed world, then remove/reorder chunks. Leaves
            // must still be global64 groups, not separate per-chunk groups.
            let seeded = Arc::new(original.prepared_triangle_range(80..130).unwrap());
            let reordered = PreparedTriangles::concatenate(&[seeded, Arc::clone(&chunks[0])]);
            let expected: Vec<_> = triangles[80..130]
                .iter()
                .chain(&triangles[..17])
                .copied()
                .collect();
            let codes = expected.iter().map(|t| t.tag as u16).collect::<Vec<_>>();
            compare(
                &native_world(expected, codes.clone(), 0.).unwrap(),
                &native_prepared_world(reordered, codes, 0.).unwrap(),
            );
            // Cached geometry still passes through the full metadata validator.
            let bad_surfaces = surfaces[..129].to_vec();
            assert_eq!(
                native_world(triangles.clone(), bad_surfaces.clone(), 0.)
                    .err()
                    .unwrap(),
                native_prepared_world(PreparedTriangles::concatenate(&chunks), bad_surfaces, 0.)
                    .err()
                    .unwrap()
            );
            for invalid in [f32::NAN, f32::INFINITY] {
                let mut bad = triangles.clone();
                bad[64].triangle.vertices[1].y = invalid;
                assert_eq!(
                    native_world(bad.clone(), surfaces.clone(), 0.)
                        .err()
                        .unwrap(),
                    native_prepared_world(PreparedTriangles::new(bad), surfaces.clone(), 0.)
                        .err()
                        .unwrap()
                );
            }
        }
        compare(
            &native_world(Vec::new(), Vec::new(), 0.).unwrap(),
            &native_prepared_world(PreparedTriangles::new(Vec::new()), Vec::new(), 0.).unwrap(),
        );
    }

    #[test]
    fn streamed_ground_keeps_current_host_car_and_object_queries() {
        use skate_dynamics::{BodyDesc, BodyType, DynamicsWorld, Shape};
        use skate3_core::math::Vector3;
        use std::sync::Arc;
        let material = skate3_core::physics::contact::RetailContactMaterial {
            static_friction: 0.,
            dynamic_friction: 0.,
            restitution: 0.,
        };
        let scene = |x: f32| Scene {
            triangles: vec![SceneTriangle {
                points: [[x, 0., 0.], [x, 0., 4.], [x + 4., 0., 0.]],
                surface: 0x3084,
                packed_surface: 0x3084,
                native_edges: None,
                native_features: Some(SceneTriangleFeatures {
                    flags: 0x1e1,
                    edge_cosines: [-1.; 3],
                }),
            }],
            ..Scene::default()
        };
        let mut world = scene(0.).world(material).unwrap();
        let mut dynamics = DynamicsWorld::default();
        let id = dynamics
            .spawn(BodyDesc {
                body_type: BodyType::Kinematic,
                shape: Shape::Box {
                    half_extents: [3., 0.5, 3.],
                },
                position: [0., 40., 0.],
                ..Default::default()
            })
            .unwrap();
        let publish = |world: &mut skate3_core::physics::board_world::BoardWorld,
                       dynamics: &DynamicsWorld| {
            world.set_external_queries(Some(Arc::new(
                crate::skate3_native::modding::moving_queries::MovingQueries::new(dynamics.solid_bodies(), Vec::new()),
            )));
        };
        let endpoints = |x: f32| (Vector3::new(x, 41., 0.), Vector3::new(x, 39., 0.));
        publish(&mut world, &dynamics);
        let (start, end) = endpoints(0.);
        let original = world.external_line(start, end, 0.).unwrap();
        assert_eq!(original.geometry_id, 0x8000_0000 | (id as u32));
        world.replace_static_geometry(scene(10.).world(material).unwrap());
        assert_eq!(
            world.external_line(start, end, 0.).unwrap().frame,
            original.frame
        );
        assert!(
            !world
                .external_nearby(Vector3::new(0., 40., 0.), 2.)
                .is_empty()
        );
        assert!(
            world
                .query_thin_line(Vector3::new(1., 1., 1.), Vector3::new(1., -1., 1.))
                .unwrap()
                .is_none()
        );
        assert!(
            world
                .query_thin_line(Vector3::new(11., 1., 1.), Vector3::new(11., -1., 1.))
                .unwrap()
                .is_some()
        );
        // Refresh the moving snapshot before another terrain publication, in
        // the actual selected-player schedule's order. No old pose may survive.
        assert!(dynamics.set_pose(id, [10., 40., 0.], [0., 0., 0., 1.]));
        publish(&mut world, &dynamics);
        world.replace_static_geometry(scene(20.).world(material).unwrap());
        assert!(world.external_line(start, end, 0.).is_none());
        let (start, end) = endpoints(10.);
        let moved = world.external_line(start, end, 0.).unwrap();
        assert_eq!(moved.geometry_id, original.geometry_id);
        assert_ne!(moved.frame, original.frame);
        world.set_external_queries(None);
        world.replace_static_geometry(scene(0.).world(material).unwrap());
        assert!(world.external_line(start, end, 0.).is_none());
    }

    #[test]
    fn skate3_line_queries_keep_audio_physics_and_pattern_bits() {
        // The original S2 fixture road surface is 0x3084: audio4, physics1,
        // pattern3. S3's wheel/feet readers consume these same packed bits.
        let scene = Scene { triangles: vec![SceneTriangle {
            points: [[0., 0., 0.], [0., 0., 4.], [4., 0., 0.]],
            surface: 0x3084, packed_surface: 0x3084, native_edges: None,
            native_features: Some(SceneTriangleFeatures { flags: 0x1e1, edge_cosines: [-1.; 3] }),
        }], ..Scene::default() };
        let world = scene.world(skate3_core::physics::contact::RetailContactMaterial {
            static_friction: 0., dynamic_friction: 0., restitution: 0.,
        }).unwrap();
        let hit = world.query_thin_line(skate3_core::math::Vector3::new(1., 1., 1.),
            skate3_core::math::Vector3::new(1., -1., 1.)).unwrap().unwrap();
        assert_eq!(hit.tag & 0x7f, 4);
        assert_eq!((hit.tag >> 7) & 31, 1);
        assert_eq!((hit.tag >> 12) & 15, 3);
    }
}
