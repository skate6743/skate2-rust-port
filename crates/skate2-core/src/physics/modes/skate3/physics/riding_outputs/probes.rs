//! The fifth/sixth board probes, kept in the same query/publication phase as
//! the wheel probes. Geometry is the world's recovered triangle dispatcher.
use skate3_core::{
    math::Vector3,
    physics::{
        board::BodyId,
        board_ground::WheelLine,
        board_probes::{self, BoardProbeHit, BoardProbeState, WallLineInput},
        board_runtime::BoardRuntime,
        board_toolkit::BoardToolkit,
        board_world::{BoardWorld, WorldLineHit},
    },
    player::input_phase::ProcessedPhysicsInput,
};

pub(crate) struct BoardProbes {
    pub deck: BoardProbeState,
    pub wall: BoardProbeState,
    wall_line: Option<WheelLine>,
    pending: Option<Pending>,
}
struct Pending {
    deck: Option<BoardProbeHit>,
    wall: Option<Option<BoardProbeHit>>,
}
impl Default for BoardProbes {
    fn default() -> Self {
        //Skateboard ctor82C0111C starts optional wall line byte291 clear.
        Self {
            deck: BoardProbeState::default(),
            wall: BoardProbeState::default(),
            wall_line: None,
            pending: None,
        }
    }
}
impl BoardProbes {
    ///ResetBoardBody82C0D680 clears published probe records, preserving
    ///query handles204C/2050 and the wall-query enable bit.
    pub fn reset_results(&mut self) {
        self.deck = BoardProbeState::default();
        self.wall = BoardProbeState::default();
    }
    pub fn wall_contact_frame(
        &self,
        time2752: f32,
        time2756: f32,
    ) -> skate3_core::riding::grounded::state::board_types::GroundContactFrame {
        let v = |p: skate3_core::math::Vector3| [p.x, p.y, p.z, 0.];
        skate3_core::riding::grounded::state::board_types::GroundContactFrame {
            vector_8032: v(self.wall.start),
            vector_8048: v(self.wall.point),
            vector_8064: v(self.wall.normal),
            word_8080: self.wall.surface_tag,
            flag_8084: self.wall.hit,
            scalar_2752: time2752,
            scalar_2756: time2756,
        }
    }
    /// PostPhysics82C02138 calls82C01F10 before updating board contact output.
    /// Its saved endpoints and enabled291 feed the NEXT StartBoard82DB6310.
    /// Processed112 is the input phase's cached deck translation, published by
    ///82C01460, not the newly solved body's current transform.
    pub fn prepare_wall(&mut self, input: &ProcessedPhysicsInput, toolkit: &BoardToolkit) {
        let vector = |value: [f32; 4]| Vector3::new(value[0], value[1], value[2]);
        self.wall_line = board_probes::wall_probe(WallLineInput {
            state: input.state_2508,
            contact_normal: vector(input.vectors_464_480_496_512_528[0].map(f32::from_bits)),
            skater_up: vector(input.vectors_544_560_592_608[0].map(f32::from_bits)),
            deck_position: vector(toolkit.deck[3]),
            time: input.time_on_ground_2752,
        });
    }
    pub fn start(&mut self, board: &BoardRuntime, world: &BoardWorld) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("Board probes submitted twice without publication".into());
        }
        let line =
            board_probes::deck_probe(board.part_transforms()[BodyId::Deck.index()].translation);
        self.deck.start(line.start);
        let deck = world
            .query_swept_line(line.start, line.end, board_probes::DECK_PROBE_RADIUS)
            .map_err(|e| format!("Stock deck ground probe: {e}"))?
            .map(hit);
        let wall = if let Some(line) = self.wall_line {
            self.wall.start(line.start);
            Some(
                world
                    .query_thin_line(line.start, line.end)
                    .map_err(|e| format!("Stock wall floor probe: {e}"))?
                    .map(hit),
            )
        } else {
            self.wall.disable();
            None
        };
        self.pending = Some(Pending { deck, wall });
        Ok(())
    }
    pub fn publish(&mut self) -> Result<(), String> {
        let pending = self
            .pending
            .take()
            .ok_or("Board probe publication requires a submitted batch")?;
        self.deck.publish(pending.deck);
        if let Some(result) = pending.wall {
            self.wall.publish(result);
        } else {
            self.wall.disable();
        }
        Ok(())
    }
}
fn hit(hit: WorldLineHit) -> BoardProbeHit {
    BoardProbeHit {
        point: hit.geometry.position,
        normal: hit.geometry.normal,
        surface_tag: hit.tag,
    }
}
