//! PhysicalPlayerHiLOD StartSkeletonLineTests82DB63C0 / result publication
//!82DB6580. These are radius-bearing trajectory tests from actual physical parts.
use skate3_core::{
    camera::TrajectoryQuery,
    math::Vector3,
    physics::{
        board_world::BoardWorld,
        skeleton_body::SkeletonBody,
        triangle_query::{TriangleLineHit, triangle_segment},
    },
    player::input_phase::{LineTestFields, PlayerInputState},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct SkeletonLineHit {
    /// Exact submitted origin, retained through completion for air-gap scoring.
    pub start_position: [f32; 4],
    /// Result0 and32, copied to the physical query record by82DB65E0/F0.
    pub position: [f32; 4],
    pub normal: [f32; 4],
    /// Result132 and collisionTime48>=0, not an arbitrary geometry-present flag.
    pub surface: u32,
    pub hit: bool,
    pub collision_time: f32,
}
impl SkeletonLineHit {
    fn pending_result() -> Self {
        // Valid test result initialized by8276C9D8: position0, normalY,
        //time-1 and surface132=0. A no-hit result still has this real record.
        Self {
            start_position: [0.0; 4],
            position: [0.0; 4],
            normal: [0.0, 1.0, 0.0, 0.0],
            surface: 0,
            hit: false,
            collision_time: -1.0,
        }
    }
}

pub(crate) struct SkeletonLineTests {
    pub hips: SkeletonLineHit,
    pub feet: [SkeletonLineHit; 2],
}
impl SkeletonLineTests {
    /// EndSkeletonLineTests82DB6580 publishes the completed tests in the
    /// order hips, left toe, right toe. ProcessInput copies these retained
    /// records into Processed1056/960/1008 during its input phase.
    pub fn publish(&self, player: &mut PlayerInputState) {
        let fields = |hit: SkeletonLineHit| LineTestFields {
            position: hit.position.map(f32::to_bits),
            normal: hit.normal.map(f32::to_bits),
            surface: hit.surface,
            valid: u8::from(hit.collision_time >= 0.0),
        };
        player.hips_line_test_1488 = fields(self.hips);
        player.left_line_test_1536 = fields(self.feet[0]);
        player.right_line_test_1584 = fields(self.feet[1]);
    }
}

pub(crate) fn query(world: &BoardWorld, body: &SkeletonBody) -> Result<SkeletonLineTests, String> {
    let parts = body.part_transforms();
    //82DB63EC..647C selects hips23,lefttoe15,righttoe19 from the
    //physical pose. Body COM and the reparented animated targets are different.
    let hips = query_trajectory(world, parts[23][3], [0.0, -100.0, 0.0, 0.0])?;
    let raised = [0.0, f32::from_bits(0x3EA8_F5C3), 0.0, 0.0];
    let lowered = [0.0, -1.5, 0.0, 0.0];
    //Keep the subtraction and subsequent FMA evaluation separate: do not
    //replace this with an independently rounded literal -1.83 or finalY-1.5.
    let velocity = std::array::from_fn(|lane| lowered[lane] - raised[lane]);
    let mut feet = [SkeletonLineHit::pending_result(); 2];
    for (foot, part) in [15, 19].into_iter().enumerate() {
        let start = std::array::from_fn(|lane| parts[part][3][lane] + raised[lane]);
        feet[foot] = query_trajectory(world, start, velocity)?;
    }
    Ok(SkeletonLineTests { hips, feet })
}

fn query_trajectory(
    world: &BoardWorld,
    start: [f32; 4],
    velocity: [f32; 4],
) -> Result<SkeletonLineHit, String> {
    //82E0B4E0 -> TestBatchManager backend virtual28. Its ctor82E0B460
    //uses Island virtual84=82764CF8, then82775E48/8276E010 creates
    //ClusteredMeshTrajectoryBatch vtable8231093C, AddTest8276E280.
    //S2 named827BDCC0 corroborates radius/time/error fields; TU3 preserves
    //f1..f4 through AggregateTrajectoryBatch8276EDD0 when multiple batches exist.
    let request = TrajectoryQuery {
        position: start,
        velocity,
        gravity: [0.0; 4],
        duration: 1.0,
        radius: f32::from_bits(0x3CF5_C28F),
        start_error: 0.0,
        end_error: 0.0,
    };
    let mut result = SkeletonLineHit::pending_result();
    result.start_position = start;
    result.collision_time = request.collision_time(|start, end, radius| {
        let origin = vector(start);
        let direction = Vector3::new(end[0] - start[0], end[1] - start[1], end[2] - start[2]);
        let mut nearest = f32::MAX;
        let mut position = None;
        for (_, triangle) in world.line_candidates(origin, vector(end), radius) {
            let mut geometry = TriangleLineHit {
                position: Vector3::ZERO,
                normal: Vector3::ZERO,
                fraction: 0.0,
                volume_parameter: [0.0; 3],
            };
            //Same source82771D08 leaf as camera: zero triangle fatness,
            //authored face normal, fraction clamp and first equal hit retained.
            if triangle_segment(
                &mut geometry,
                origin,
                direction,
                triangle.triangle.vertices,
                radius,
                0.0,
            ) {
                let lower = if -geometry.fraction >= 0.0 {
                    0.0
                } else {
                    geometry.fraction
                };
                let fraction = if 1.0 - lower >= 0.0 { lower } else { 1.0 };
                if fraction < nearest {
                    nearest = fraction;
                    let point = lanes(geometry.position);
                    result.position = point;
                    result.normal = lanes(triangle.triangle.feature.normal);
                    result.surface = triangle.tag;
                    position = Some(point);
                }
            }
        }
        if let Some(external) = world.external_line(origin, vector(end), radius) {
            let hit = external.hit.geometry;
            if hit.fraction.clamp(0.0, 1.0) < nearest {
                let point = lanes(hit.position);
                result.position = point;
                result.normal = lanes(hit.normal);
                result.surface = external.hit.tag;
                position = Some(point);
            }
        }
        Ok::<_, String>(position)
    })?;
    result.hit = result.collision_time >= 0.0;
    Ok(result)
}
fn vector(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
fn lanes(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}

#[cfg(any())]
mod mod_platform_tests {
    use super::*;
    use std::sync::Arc;
    use skate_dynamics::{DynamicsWorld, BodyDesc, BodyType, Shape};
    #[test]
    fn toes_and_hips_find_rapier_platform_and_follow_its_pose() {
        let mut dynamics = DynamicsWorld::default();
        let id = dynamics.spawn(BodyDesc { body_type: BodyType::Kinematic,
            shape: Shape::Box { half_extents: [3.,0.5,3.] }, position:[0.,40.,0.],
            ..Default::default() }).unwrap();
        let mut world = BoardWorld::new(vec![]);
        world.set_external_queries(Some(Arc::new(crate::skate3_native::modding::moving_queries::MovingQueries::new(dynamics.solid_bodies(),Vec::new()))));
        for velocity in [[0.,-1.83,0.,0.],[0.,-100.,0.,0.]] {
            let hit=query_trajectory(&world,[0.,41.,0.,0.],velocity).unwrap();
            assert!(hit.hit); assert!((hit.position[1]-40.5).abs()<0.001);
            assert!(hit.normal[1]>0.99);
        }
        assert!(!query_trajectory(&world,[8.,41.,0.,0.],[0.,-1.83,0.,0.]).unwrap().hit);
        assert!(dynamics.set_pose(id,[10.,40.,0.],[0.,0.,0.,1.]));
        world.set_external_queries(Some(Arc::new(crate::skate3_native::modding::moving_queries::MovingQueries::new(dynamics.solid_bodies(),Vec::new()))));
        assert!(!query_trajectory(&world,[0.,41.,0.,0.],[0.,-1.83,0.,0.]).unwrap().hit);
        assert!(query_trajectory(&world,[10.,41.,0.,0.],[0.,-1.83,0.,0.]).unwrap().hit);
    }
}
