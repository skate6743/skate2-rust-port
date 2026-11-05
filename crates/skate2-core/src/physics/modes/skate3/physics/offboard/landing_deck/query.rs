//! Real accelerated trajectory query and ordered completion of82D79948/79B40.
use super::{Owner, input};
use crate::skate3_native::physics::{offboard::contact_toolkit::StaticScene, player_input::PlayerInputRuntime};
use skate3_core::player::{
    input_phase::ProcessedPhysicsInput,
    offboard::{
        hippy_jump,
        landing_deck::{QueryRequest, QueryResult, SyncInput},
    },
};

pub(super) fn execute(
    scene: &StaticScene<'_>,
    request: QueryRequest,
    processed: &ProcessedPhysicsInput,
) -> Result<QueryResult, String> {
    //79948 stack54=processed2952; stack5C=0; stack64=null.
    //8276E280 writes request80/84 from stack54/5C.8276DBE0 forwards
    //these to82770B40's matching-group and mesh-rejection checks.
    //Use the accelerated arc provider, NOT Ground's sweep-oriented Scene batch.
    scene
        .trajectory(request, processed.actor_query_2952 as i32, 0)
        .map_err(str::to_owned)
}

impl Owner {
    ///Run the actual query before acknowledging native pending262.
    ///The returned real result remains private until PostPhysics.
    pub(crate) fn submit(
        &mut self,
        scene: &StaticScene<'_>,
        request: QueryRequest,
        processed: &ProcessedPhysicsInput,
    ) -> Result<(), String> {
        let completion = execute(scene, request, processed)?;
        self.completion = Some(completion);
        self.manager.query_submitted();
        Ok(())
    }

    ///Native caller tests262 before Sync. Host completes work synchronously at
    ///submission but preserves its consumption phase; this is not a claim of
    ///the console job backend's timing or allocation semantics.
    pub(crate) fn post_physics(&mut self, input: &PlayerInputRuntime) -> Result<(), String> {
        if !self.manager.pending_262 {
            if self.completion.is_some() {
                return Err("Landing query completion exists without native pending262".into());
            }
            return Ok(());
        }
        let completion = self
            .completion
            .ok_or("Landing pending262 has no submitted world-query completion")?;
        let p = &input.processed;
        self.manager.sync(
            completion.valid().then_some(completion),
            &SyncInput {
                position_592: input::position(input),
                flags_2488: p.flags_2488,
            },
            |_| {
                //Only the original79E30 gate inside Sync invokes this existing
                //numeric provider. No state503, custom branch, or synthetic result.
                let processed = input::processed(input)?;
                Ok::<_, String>(hippy_jump::calculate(hippy_jump::Input {
                    desired_height: f32::from_bits(p.vectors_880_896_912_928_944[3][1])
                        + f32::from_bits(0x3e4c_cccd),
                    board_position: processed.board_position_112,
                    centre_of_mass_position: input::position(input),
                    reference_up: processed.up_544,
                    current_velocity: processed.board_velocity_400,
                }))
            },
        )?;
        self.completion = None;
        Ok(())
    }
}
