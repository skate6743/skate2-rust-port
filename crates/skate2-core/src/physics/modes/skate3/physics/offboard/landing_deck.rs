//! Offboard host for original LandingOnDeckManager82D78B28..82D79E30.
//! One retained manager and real scene-query completion; no physical-state facade.
mod input;
mod output;
mod query;
use super::contact_toolkit::StaticScene;
use crate::skate3_native::physics::{animated_skeleton::AnimatedSkeleton, player_input::PlayerInputRuntime};
pub(crate) use output::PublishTargets;
use skate3_core::player::offboard::landing_deck::{
    self as native, AssistInput, IkInput, Manager, QueryResult, Settings, UpdateOutput, Vector,
};
use skate_data::collections::Collections;

pub(crate) struct Owner {
    ///Air and transitions must mutate this same manager, including Reset/force.
    pub manager: Manager,
    pub settings: Settings,
    ///Some is an actually executed query, including an actual miss.
    ///Retained until the ordered PostPhysics phase, never a fabricated seed.
    completion: Option<QueryResult>,
}
impl Owner {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            manager: Manager::default(),
            settings: Settings {
                deck_min_uprightness: data.float(
                    "physics_landingondeck",
                    "default",
                    "DeckMinUprightness",
                )?,
                approximate_com_height: data.float(
                    "physics_landingondeck",
                    "default",
                    "ApproxCOMHeightOnLanding",
                )?,
            },
            completion: None,
        })
    }

    ///82D78B28 deliberately retains proposed trajectory AND pending completion.
    pub(crate) fn reset(&mut self) {
        self.manager.reset();
    }

    ///82D78D30. Query failure propagates without committing the speculative state.
    pub(crate) fn assist(
        &mut self,
        scene: &StaticScene<'_>,
        input: &PlayerInputRuntime,
        maximum_velocity_change: f32,
    ) -> Result<(), String> {
        let mut next = self.manager;
        let request = next.assist(
            &AssistInput {
                processed: input::processed(input)?,
                position: input::position(input),
                velocity: input::velocity(input),
                maximum_velocity_change,
            },
            &self.settings,
        );
        let completion = request
            .map(|request| query::execute(scene, request, &input.processed))
            .transpose()?;
        if let Some(completion) = completion {
            //Native Assist starts a new batch even if an older one was pending.
            self.completion = Some(completion);
            next.query_submitted();
        }
        self.manager = next;
        Ok(())
    }

    ///82D794A0: returns EVERY numeric output. A returned query has already run;
    ///the caller must not submit UpdateOutput.query a second time.
    pub(crate) fn update(
        &mut self,
        scene: &StaticScene<'_>,
        input: &PlayerInputRuntime,
    ) -> Result<UpdateOutput, String> {
        let mut next = self.manager;
        let output = next.update(&input::processed(input)?, &self.settings);
        let completion = output
            .query
            .map(|request| query::execute(scene, request, &input.processed))
            .transpose()?;
        if let Some(completion) = completion {
            self.completion = Some(completion);
            next.query_submitted();
        }
        self.manager = next;
        Ok(output)
    }

    ///82D792D0. mapped_position_12608 is the PRE-adjustment mapped board
    ///translation copied by82BD8CB0..8D14, not animation_board after offsets.
    ///The shared pose producer supplies that retained value explicitly.
    pub(crate) fn calculate_accurate_ik_offset(
        &mut self,
        input: &PlayerInputRuntime,
        animation: &AnimatedSkeleton,
        mapped_position_12608: Vector,
        time_to_land: f32,
    ) -> Result<Vector, String> {
        Ok(self.manager.calculate_accurate_ik_offset(&IkInput {
            processed: input::processed(input)?,
            time_to_land,
            animation_root: animation.roots.animation_to_world,
            animation_com_10960: animation.record.centre_of_mass,
            mapped_position_12608,
        }))
    }

    pub(crate) fn fill(&self) -> native::FillOutput {
        self.manager.fill()
    }
}
