//! Ground Sync submits edge geometry; the next PreUpdate consumes its result.
use super::ground_sync::SceneService;
use skate3_core::{
    math::Vector3,
    physics::board_world::BoardWorld,
    player::offboard::ground_query::{
        self as query, ConsumeInput, Frame, GroundAdjustment, GroundQueryPacket, GroundQueryScene,
        LineHit, QueryContext,
    },
};
use skate_data::collections::Collections;

pub(crate) struct State {
    pending: Option<(GroundQueryPacket, [Option<LineHit>; 7])>,
    collision_offset: f32,
}
impl State {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            pending: None,
            //82C209E4 and Skate2 GetGeometryCollisionOffset82CFABD8:
            //physics_grinds layout636, full keyCBFEFFEA12F4CC27.
            collision_offset: data.float("physics_grinds", "default", "DeckCenterToTruck")?,
        })
    }
    pub(crate) fn reset(&mut self) {
        self.pending = None;
    }
    pub(crate) fn consume(&mut self, input: ConsumeInput) -> GroundAdjustment {
        let geometry = self
            .pending
            .take()
            .map(|(packet, hits)| query::interpret_hits(&packet, hits));
        query::consume_geometry(input, geometry)
    }
    pub(crate) fn submit(
        &mut self,
        world: &BoardWorld,
        vehicles: &[(usize, skate_dynamics::SolidBody)],
        frame: [[f32; 4]; 4],
        velocity: [f32; 4],
        context: QueryContext,
        flags_2488: u32,
    ) -> Result<(), String> {
        let frame = query_frame(frame);
        let search = query::edge_search(frame, context, xyz(velocity), flags_2488);
        let mut service = SceneService { world, vehicles };
        let candidates = service.edge_candidates(&search)?;
        if let Some(packet) = query::select_edge(search, &candidates)
            .and_then(|edge| query::prepare_packet(frame, context, edge, self.collision_offset))
        {
            //82C20BF0 starts the real batch during Sync. Host execution may be
            //synchronous, but interpretation/publication waits for PreUpdate.
            let hits = service.query_lines(&packet)?;
            self.pending = Some((packet, hits));
        }
        //82D321CC: no selected edge means no submit, not a fabricated query.
        Ok(())
    }
}
pub(crate) fn query_frame(frame: [[f32; 4]; 4]) -> Frame {
    Frame {
        right: xyz(frame[0]),
        up: xyz(frame[1]),
        forward: xyz(frame[2]),
        position: xyz(frame[3]),
    }
}
pub(crate) fn xyz(v: [f32; 4]) -> Vector3 {
    Vector3::new(v[0], v[1], v[2])
}
pub(crate) fn lanes(v: Vector3) -> [f32; 4] {
    [v.x, v.y, v.z, 0.]
}
pub(crate) fn native_frame(frame: Frame) -> [[f32; 4]; 4] {
    [
        lanes(frame.right),
        lanes(frame.up),
        lanes(frame.forward),
        lanes(frame.position),
    ]
}
