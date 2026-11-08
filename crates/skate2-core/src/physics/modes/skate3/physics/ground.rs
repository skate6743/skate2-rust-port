//! Our level geometry, queried with the recovered physics/world calculations.
use skate3_core::{
    math::Vector3,
    physics::{
        board_world::{
            BoardWorld, ContactRetentionSettings, WorldTriangle,
            query_metadata::{Bounds, EdgeSegment, QueryMesh, QueryMetadata, QueryPool},
        },
        collision::{TriangleFeature, WorldContactSettings},
        contact::RetailContactMaterial,
        drive_frames::RetailAffineTransform,
        world_contact::triangle_from_volume,
    },
};

pub(crate) const HEIGHT: f32 = -0.035;

/// One authored terrain selection drives both presentation and live queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Terrain {
    #[cfg(any())]
    Flat,
    Course,
}

impl Terrain {
    #[cfg(any())]
    pub(crate) fn surfaces(self) -> [Vec<[Vector3; 4]>; 4] {
        match self { Self::Flat => flat_surfaces(), Self::Course => surfaces() }
    }
    pub(crate) fn world(self, material: RetailContactMaterial) -> BoardWorld {
        match self {
            #[cfg(any())]
            Self::Flat => flat_world(material),
            Self::Course => world(material),
        }
    }
}

/// Authored level dimensions in metres; HEIGHT remains the starting surface.
pub(crate) const FLOOR_HEIGHT: f32 = HEIGHT - 1.5;

/// Shared quads for rendering and collision: landing floor, starting box, half pipe.
/// Quads use the same winding as the original floor (0,2,1 and 0,3,2).
pub(crate) fn surfaces() -> [Vec<[Vector3; 4]>; 4] {
    let v = Vector3::new;
    let floor = vec![[
        v(-50.0, FLOOR_HEIGHT, -50.0),
        v(50.0, FLOOR_HEIGHT, -50.0),
        v(50.0, FLOOR_HEIGHT, 50.0),
        v(-50.0, FLOOR_HEIGHT, 50.0),
    ]];
    let top = [
        v(-3.0, HEIGHT, -4.0),
        v(3.0, HEIGHT, -4.0),
        v(3.0, HEIGHT, 4.0),
        v(-3.0, HEIGHT, 4.0),
    ];
    let mut platform = vec![top];
    for i in [0, 1, 3] {
        // Forward face opens directly onto the ramp.
        let a = top[i];
        let b = top[(i + 1) % 4];
        platform.push([b, a, v(a.x, FLOOR_HEIGHT, a.z), v(b.x, FLOOR_HEIGHT, b.z)]);
    }

    // Full-width downhill route straight ahead from spawn. Ease both ends
    // into the horizontal surfaces so the crest is not a sharp launch edge.
    let ramp_segments = 48;
    let ramp_length = 12.0;
    let ramp_point = |i: usize| {
        let t = i as f32 / ramp_segments as f32;
        let blend = t * t * (3.0 - 2.0 * t);
        (
            4.0 + ramp_length * t,
            HEIGHT + (FLOOR_HEIGHT - HEIGHT) * blend,
        )
    };
    for i in 0..ramp_segments {
        let (z0, y0) = ramp_point(i);
        let (z1, y1) = ramp_point(i + 1);
        platform.push([
            v(-3.0, y0, z0),
            v(3.0, y0, z0),
            v(3.0, y1, z1),
            v(-3.0, y1, z1),
        ]);
    }
    platform.extend(crate::skate3_native::grind_world::surfaces());
    // Open ends at z +/-6 allow entry from the landing floor. Circular
    // transitions meet the four-metre flat bottom tangentially.
    let radius = 2.5;
    let segments = 32;
    let mut profile = Vec::new();
    for i in (0..=segments).rev() {
        let angle = std::f32::consts::FRAC_PI_2 * i as f32 / segments as f32;
        profile.push((
            12.0 - 2.0 - radius * angle.sin(),
            FLOOR_HEIGHT + radius * (1.0 - angle.cos()),
        ));
    }
    for i in 0..=segments {
        let angle = std::f32::consts::FRAC_PI_2 * i as f32 / segments as f32;
        profile.push((
            12.0 + 2.0 + radius * angle.sin(),
            FLOOR_HEIGHT + radius * (1.0 - angle.cos()),
        ));
    }
    let mut half_pipe = Vec::new();
    for pair in profile.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        // The flat bottom already belongs to the landing floor.
        if y0 == FLOOR_HEIGHT && y1 == FLOOR_HEIGHT {
            continue;
        }
        half_pipe.push([
            v(x0, y0, -6.0),
            v(x1, y1, -6.0),
            v(x1, y1, 6.0),
            v(x0, y0, 6.0),
        ]);
    }
    // One-metre decks at both lips.
    for (x0, x1) in [(6.5, 7.5), (16.5, 17.5)] {
        let y = FLOOR_HEIGHT + radius;
        half_pipe.push([v(x0, y, -6.0), v(x1, y, -6.0), v(x1, y, 6.0), v(x0, y, 6.0)]);
    }
    // A separate 2.2 m ledge on the left of the course makes the custom
    // climbing flow directly testable without altering the riding routes.
    let climb_top = [
        v(-9., FLOOR_HEIGHT + 2.2, -2.),
        v(-5., FLOOR_HEIGHT + 2.2, -2.),
        v(-5., FLOOR_HEIGHT + 2.2, 2.),
        v(-9., FLOOR_HEIGHT + 2.2, 2.),
    ];
    let mut climbing = vec![climb_top];
    for i in 0..4 {
        let a = climb_top[i];
        let b = climb_top[(i + 1) % 4];
        climbing.push([b, a, v(a.x, FLOOR_HEIGHT, a.z), v(b.x, FLOOR_HEIGHT, b.z)]);
    }
    [floor, platform, half_pipe, climbing]
}

pub(super) fn world(material: RetailContactMaterial) -> BoardWorld {
    build_world(material, surfaces(), query_edges())
}

/// Deliberately authored flat-ground comparison surface. Both the solver and
/// all gameplay probes query these real collision triangles.
#[cfg(any())]
pub(crate) fn flat_surfaces() -> [Vec<[Vector3; 4]>; 4] {
    let v = Vector3::new;
    [
        vec![[
            v(-50.0, HEIGHT, -50.0),
            v(50.0, HEIGHT, -50.0),
            v(50.0, HEIGHT, 50.0),
            v(-50.0, HEIGHT, 50.0),
        ]],
        vec![],
        vec![],
        vec![],
    ]
}
#[cfg(any())]
pub(crate) fn flat_world(material: RetailContactMaterial) -> BoardWorld {
    build_world(material, flat_surfaces(), Vec::new())
}

fn build_world(
    material: RetailContactMaterial,
    surfaces: [Vec<[Vector3; 4]>; 4],
    edges: Vec<EdgeSegment>,
) -> BoardWorld {
    let mut triangles = Vec::new();
    let rail_faces = crate::skate3_native::grind_world::surfaces();
    for (tag, quads) in surfaces.into_iter().enumerate() {
        for vertices in quads {
            let rail_face = rail_faces.contains(&vertices);
            for (half, indices) in [[0, 2, 1], [0, 3, 2]].into_iter().enumerate() {
                let convex = if rail_face { if half == 0 { 0x60 } else { 0xc0 } } else { 0 };
                triangles.push(WorldTriangle {
                    triangle: triangle_from_volume(
                        indices.map(|i| vertices[i]),
                        0.0,
                        [1.0; 3],
                        TriangleFeature::ONE_SIDED | convex,
                    ),
                    material,
                    tag: tag as u32,
                });
            }
        }
    }
    //This test level explicitly authors material0/category0/upper flags0.
    //This is host metadata, not a claim that the stock material is concrete,
    //and not derived from the neutral contact coefficients or rendering tag.
    let metadata = QueryMetadata {
        packed_surfaces: vec![0u16; triangles.len()],
        meshes: vec![QueryMesh {
            triangle_range: 0..triangles.len(),
            local_to_world: RetailAffineTransform::IDENTITY,
            world_to_local: RetailAffineTransform::IDENTITY,
            local_bounds: Bounds::from_points(
                triangles.iter().flat_map(|entry| entry.triangle.vertices),
            )
            .expect("Authored level contains collision triangles"),
            matching_group: -1,
            // This authored static collision mesh accepts trajectory queries.
            rejection_flags: 0,
            geometry: 1,
            pool: QueryPool::Ground,
        }],
        static_edges: edges,
        //No island or conditional meshes exist in the current static level.
        island_flags: 0,
    };
    BoardWorld::with_query_metadata(triangles, metadata)
        .expect("Authored level query metadata matches its collision geometry")
}
///Deliberately authored ledge records, in level order. Neither the triangle
///diagonals nor the smooth ramp/half-pipe tessellation seams are ledges.
fn query_edges() -> Vec<EdgeSegment> {
    let v = Vector3::new;
    let top = [
        v(-3.0, HEIGHT, -4.0),
        v(3.0, HEIGHT, -4.0),
        v(3.0, HEIGHT, 4.0),
        v(-3.0, HEIGHT, 4.0),
    ];
    let mut edges: Vec<_> = [0, 1, 3]
        .into_iter()
        .map(|i| (top[i], top[(i + 1) % 4]))
        .collect();
    let lip_height = FLOOR_HEIGHT + 2.5;
    for x in [7.5, 16.5] {
        edges.push((v(x, lip_height, -6.0), v(x, lip_height, 6.0)));
    }
    edges
        .into_iter()
        .map(|(start, end)| EdgeSegment {
            start,
            end,
            local_bounds: Bounds::from_points([start, end]).expect("Two authored edge endpoints"),
        })
        .collect()
}

/// A network game's skater world contacts per tick (Step_Collision1 827641A0..4318): the
/// online skater ground job gets the cap 0x51 - 1 = 80 (827642E0..F8), its one buffer for
/// the board and skeleton assemblies together, inside the visual island's 80-contact
/// ContactCacheWrapper (827631C8 slot 20, 82765AA0). Offline the job writes the solver's
/// contact cache with no cap (-1).
pub(super) const ONLINE_SKATER_GROUND_CONTACTS: u32 = 80;

pub(super) fn query_settings() -> (WorldContactSettings, ContactRetentionSettings) {
    (
        WorldContactSettings {
            // Serialized primitive82DC3AE4/82DC3B84, source literal82165A00.
            volume_padding: 0.05,
            // Ground job8277C864/8277C86C; board selection82768728.
            maximum_separating_distance: 0.5,
            edge_cos_bend_normal_threshold: 0.999,
            convexity_epsilon: 0.01,
            is_object: false,
        },
        ContactRetentionSettings {
            capacity: u32::MAX, // Host storage capacity, no artificial row limit.
            // Step_Collision1 8276430C ->827771E8 ->82777384 ->8277C664.
            duplicate_distance_squared: -1.0,
            deferred_reduction: true,
        },
    )
}

#[cfg(any())]
#[path = "tests/flat_ground.rs"]
mod flat_tests;
