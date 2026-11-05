//! Native numerical callbacks for the ordinary ground-state branch.
use super::GroundRuntime;
use skate3_core::{
    math::Vector3,
    physics::{
        board::BodyId, board_runtime::BoardRuntime, manual::controller::ManualAngleMeasurement,
    },
    riding::{
        collision_response::signed_angle, ground_correction_math,
        grounded::state::corrections::AntiFlipNudgeMath,
    },
};
use std::convert::Infallible;

impl ManualAngleMeasurement for GroundRuntime {
    type Error = Infallible;
    fn angle_between(
        &mut self,
        deck_z: [f32; 4],
        reference_z: [f32; 4],
        reference_x: [f32; 4],
    ) -> Result<f32, Self::Error> {
        Ok(signed_angle(
            xyz(deck_z),
            xyz(reference_z),
            xyz(reference_x),
        ))
    }
}
impl AntiFlipNudgeMath for GroundRuntime {
    type Error = Infallible;
    fn dot3(&mut self, a: [f32; 4], b: [f32; 4]) -> Result<f32, Self::Error> {
        Ok(ground_correction_math::dot_product(a, b))
    }
    fn scale_to_magnitude(
        &mut self,
        vector: [f32; 4],
        squared_length: f32,
        magnitude: f32,
    ) -> Result<[f32; 4], Self::Error> {
        Ok(ground_correction_math::scale_to_magnitude(
            vector,
            squared_length,
            magnitude,
        ))
    }
}
impl GroundRuntime {
    pub fn detect_hung_up_geometry(
        &self,
        world: &skate3_core::physics::board_world::BoardWorld,
        input: skate3_core::physics::ground_hang_geometry::HangGeometryInput,
    ) -> Result<bool, String> {
        skate3_core::physics::ground_hang_geometry::detect_hung_geometry(
            world,
            input,
            self.deck_center_to_truck,
        )
        .map_err(str::to_owned)
    }
    pub fn collision_force_projection(&self, force: [f32; 4], velocity: [f32; 4]) -> f32 {
        ground_correction_math::collision_force_projection(force, velocity)
    }
    pub fn build_hang_force(
        &self,
        board: &BoardRuntime,
        edge_start: [f32; 4],
        edge_end: [f32; 4],
    ) -> [f32; 4] {
        let p = board.part_transforms()[BodyId::Deck.index()].translation;
        ground_correction_math::hang_force(edge_start, edge_end, [p.x, p.y, p.z, 0.])
    }
    pub fn apply_hang_force(&mut self, board: &mut BoardRuntime, force: [f32; 4]) {
        let position = board.part_transforms()[BodyId::Deck.index()].translation;
        ground_correction_math::apply_world_force(
            &mut board.bodies_mut()[BodyId::Deck.index()],
            position,
            xyz(force),
            position,
        );
    }
    pub fn pin_to_position(
        &mut self,
        board: &mut BoardRuntime,
        captured_x: f32,
        captured_z: f32,
        dt: f32,
    ) {
        let p = board.part_transforms()[BodyId::Deck.index()].translation;
        let velocity = ground_correction_math::pinning_velocity(
            [p.x, p.y, p.z, 0.],
            captured_x,
            captured_z,
            dt,
        );
        self.set_animated_velocity(board, velocity);
    }
}
fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
