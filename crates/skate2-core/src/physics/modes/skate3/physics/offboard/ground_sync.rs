//! Host side scene service for the recovered BipedGround synchronizer.
//!
//! The synchronizer owns the query ordering in `skate-core`; this adapter only
//! exposes the already loaded BoardWorld.  It does not cache contacts or run a
//! second solver.  Every query therefore sees the same authored triangles and
//! packed surfaces as the board collision pass.
use super::ground_query::{self, IndexedEdgeBody};
use skate3_core::{
    physics::board_world::BoardWorld,
    player::offboard::ground_query::{EdgeSearch, GroundQueryPacket, GroundQueryScene, LineHit},
};

pub(crate) struct SceneService<'a> {
    pub world: &'a BoardWorld,
    pub grind: &'a crate::skate3_native::grind_world::StaticProvider,
    pub vehicles: &'a [(usize, skate_dynamics::SolidBody)],
}

impl GroundQueryScene for SceneService<'_> {
    type Error = &'static str;

    fn edge_candidates(
        &mut self,
        search: &EdgeSearch,
    ) -> Result<Vec<skate3_core::player::offboard::ground_query::Edge>, Self::Error> {
        let cache = super::mod_solid_ground::VehicleEdgeCache::build(self.vehicles);
        let vehicles = cache.vehicles();
        ground_query::with_world_scene(
            self.world,
            self.grind,
            cache.primary_edges(&vehicles),
            &[] as &[IndexedEdgeBody<'_>],
            |scene| scene.edge_candidates(search),
        )
    }

    fn query_lines(
        &mut self,
        packet: &GroundQueryPacket,
    ) -> Result<[Option<LineHit>; 7], Self::Error> {
        let cache = super::mod_solid_ground::VehicleEdgeCache::build(self.vehicles);
        let vehicles = cache.vehicles();
        let mut hits = ground_query::with_world_scene(
            self.world,
            self.grind,
            cache.primary_edges(&vehicles),
            &[] as &[IndexedEdgeBody<'_>],
            |scene| scene.query_lines(packet),
        )?;
        for (line, destination) in packet.lines.iter().zip(&mut hits) {
            if let Some(hit) = self.world.external_line(line.start, line.end, line.radius) {
                if destination.as_ref().is_none_or(|old| hit.hit.geometry.fraction < old.fraction) {
                    *destination = Some(LineHit {
                        position: hit.hit.geometry.position, face_normal: hit.hit.geometry.normal,
                        fraction: hit.hit.geometry.fraction, packed_surface: hit.surface,
                    });
                }
            }
        }
        Ok(hits)
    }
}
