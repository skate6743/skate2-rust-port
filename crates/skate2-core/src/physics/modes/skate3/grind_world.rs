//! Authored test-course grind paths. Collision/visuals share these dimensions.
//! This asset owner does not claim a nearby spline is an acquired grind.
use skate3_core::math::Vector3;
use crate::skate3_native::math_host::*;

/// Shared authored geometry. The physical grind owner runs native truck queries
/// against these same spline endpoints before state selection.

pub(crate) struct GrindGeometry {
    pub rails: Vec<Rail>,
    pub native_blob: Vec<u8>,
}




impl GrindGeometry {
    pub(crate) fn for_world(test_world: bool) -> Self {
        if test_world { Self { rails: rails().to_vec(), native_blob: spline_blob() } }
        else { Self { rails: Vec::new(), native_blob: Vec::new() } }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Rail {
    pub id: u64,
    pub name: &'static str,
    pub start: Vector3,
    pub end: Vector3,
    pub width: f32,
    pub thickness: f32,
}

pub(crate) fn rails() -> [Rail; 7] {
    let floor = crate::skate3_native::physics::ground::FLOOR_HEIGHT;
    let top = crate::skate3_native::physics::ground::HEIGHT;
    let v = Vector3::new;
    [
        Rail { id: 1, name: "low_flat_rail", start: v(-7., floor + 0.45, 7.),
            end: v(-7., floor + 0.45, 15.), width: 0.10, thickness: 0.10 },
        Rail { id: 2, name: "high_flat_rail", start: v(4.5, floor + 0.70, 8.),
            end: v(4.5, floor + 0.70, 16.), width: 0.10, thickness: 0.10 },
        Rail { id: 3, name: "halfpipe_left_coping", start: v(7.5, floor + 2.5, -6.),
            end: v(7.5, floor + 2.5, 6.), width: 0.08, thickness: 0.08 },
        Rail { id: 4, name: "halfpipe_right_coping", start: v(16.5, floor + 2.5, -6.),
            end: v(16.5, floor + 2.5, 6.), width: 0.08, thickness: 0.08 },
        Rail { id: 5, name: "platform_back_edge", start: v(-3., top, -4.),
            end: v(3., top, -4.), width: 0., thickness: 0. },
        Rail { id: 6, name: "platform_right_edge", start: v(3., top, -4.),
            end: v(3., top, 4.), width: 0., thickness: 0. },
        Rail { id: 7, name: "platform_left_edge", start: v(-3., top, 4.),
            end: v(-3., top, -4.), width: 0., thickness: 0. },
    ]
}

/// Box rails have their spline on the top centre, not the centre of the tube.
/// Coping is flush with the deck lip; the existing transition remains intact.
pub(crate) fn surfaces() -> Vec<[Vector3; 4]> {
    let mut out = Vec::new();
    let floor = crate::skate3_native::physics::ground::FLOOR_HEIGHT;
    for rail in rails() {
        // Ledge paths use the already authored platform collision.
        if rail.width == 0. { continue; }
        let x = rail.start.x;
        let y = rail.start.y;
        box_faces(&mut out, Vector3::new(x - rail.width * 0.5, y - rail.thickness, rail.start.z),
            Vector3::new(x + rail.width * 0.5, y, rail.end.z));
        if rail.id <= 2 {
            for z in [rail.start.z + 0.6, rail.end.z - 0.6] {
                box_faces(&mut out, Vector3::new(x - 0.04, floor, z - 0.04),
                    Vector3::new(x + 0.04, y - rail.thickness, z + 0.04));
            }
        }
    }
    out
}

fn box_faces(out: &mut Vec<[Vector3; 4]>, min: Vector3, max: Vector3) {
    let v = Vector3::new;
    let top = [v(min.x, max.y, min.z), v(max.x, max.y, min.z),
        v(max.x, max.y, max.z), v(min.x, max.y, max.z)];
    out.push(top);
    for i in 0..4 {
        let a = top[i]; let b = top[(i + 1) % 4];
        out.push([b, a, v(a.x, min.y, a.z), v(b.x, min.y, b.z)]);
    }
    out.push([v(min.x, min.y, max.z), v(max.x, min.y, max.z),
        v(max.x, min.y, min.z), v(min.x, min.y, min.z)]);
}

mod spline;
mod provider;
mod octree;
pub(crate) use provider::StaticProvider;

#[cfg(any())]
mod tests;

pub(crate) fn spline_blob() -> Vec<u8> {
    spline::build_rails(&test_rails()).expect("authored course splines")
}
pub(super) fn test_rails() -> Vec<skate_data::skate_map::Rail> {
    rails().iter().map(|r| skate_data::skate_map::Rail {
        name: r.name.into(), points: vec![[r.start.x,r.start.y,r.start.z], [r.end.x,r.end.y,r.end.z]],
        closed: false, native: None,
    }).collect()
}
