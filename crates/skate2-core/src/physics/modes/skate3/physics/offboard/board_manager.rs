//! Persistent SkateboardController adapter, not the grab-spline query manager.
//!
//! TU3 82D74FD8 creates two hand/deck drives; 82DB6150 updates possession
//! after the physical state's Update and before the common board update.
//! State448/word444/system452 are borrowed from the existing controller. Never
//! create another copy in Skate3ControllerState, or infer ownership from graph flags.
use super::board_possession::{self, Effects};
use skate3_core::{
    physics::{assembly::BodySnapshot, drive_solver::RetailDriveRows},
    player::{
        lifecycle::{SkateboardControllerActions, SkateboardControllerFields},
        offboard::board_possession::{
            self as native, Frame,
            lifecycle::{Observation, Settings},
        },
    },
};
use skate_data::collections::Collections;
pub(crate) mod runtime;

#[cfg(any())]
mod tests;

pub(crate) struct Owner {
    pub state: native::State,
    settings: Settings,
}

impl Owner {
    pub(crate) fn load(data: &Collections) -> Result<Self, String> {
        Ok(Self {
            state: native::State::default(),
            settings: board_possession::settings::load(data)?,
        })
    }

    /// Call once after the selected physical state Update. Observations must
    /// come from the evaluated skeleton and completed contact publications.
    pub(crate) fn update(
        &mut self,
        fields: &mut SkateboardControllerFields,
        observation: &Observation,
        effects: &mut Effects<'_>,
    ) {
        self.state
            .update(fields, observation, &self.settings, effects);
    }

    pub(crate) fn hold(
        &mut self,
        fields: &mut SkateboardControllerFields,
        observation: &Observation,
        effects: &mut Effects<'_>,
    ) {
        self.state.hold(fields, observation, effects);
    }

    /// LetGo does not assign state448: the physical-state lifecycle or
    /// UpdateState caller does that after disabling the physical drives.
    pub(crate) fn let_go(
        &mut self,
        fields: &mut SkateboardControllerFields,
        observation: &Observation,
        effects: &mut Effects<'_>,
    ) {
        self.state
            .let_go(fields, observation, &self.settings, effects);
    }

    pub(crate) fn stop(
        &mut self,
        fields: &mut SkateboardControllerFields,
        observation: &Observation,
        effects: &mut Effects<'_>,
    ) {
        self.state
            .stop(fields, observation, &self.settings, effects);
    }

    ///82D75AC8: the board left on the ground, animated, while BipedBoardOnGround is set.
    pub(crate) fn rest_on_ground(
        &mut self,
        fields: &mut SkateboardControllerFields,
        effects: &mut Effects<'_>,
    ) {
        self.state.rest_on_ground(fields, effects);
    }

    ///82DB8CE4..8D1C and82DB8D6C..8DA4: clear countdown even when
    ///already off, Stop, then74D30 resets retrieval only (not hand frames).
    pub(crate) fn reset_for_teleport(
        &mut self,
        fields: &mut SkateboardControllerFields,
        observation: &Observation,
        effects: &mut Effects<'_>,
    ) {
        fields.word_444 = 0;
        self.stop(fields, observation, effects);
        self.state.retrieval = native::Retrieval::default();
    }

    /// Append to the existing island's drive rows, with the SAME reaction
    /// indices used for skeleton part3/7 and deck6 by the contact/joint passes.
    /// There is no separate board simulation and no render-bone attachment.
    pub(crate) fn append_drives(
        &self,
        deck: BodySnapshot,
        hands: [BodySnapshot; 2],
        deck_reaction: usize,
        hand_reactions: [usize; 2],
        dt: f32,
        rows: &mut Vec<RetailDriveRows>,
    ) {
        board_possession::drives::append(
            &self.state,
            deck,
            hands,
            deck_reaction,
            hand_reactions,
            dt,
            rows,
        );
    }

    /// 82D76D20: pass the real Skeleton::GetBoneTransform(11), not a hand,
    /// root, deck frame or bind-pose substitute.
    pub(crate) fn fill(
        &self,
        fields: &SkateboardControllerFields,
        processed: &native::Processed,
        bone_11: Frame,
    ) -> native::Fill {
        native::fill(fields, &self.state, processed, bone_11)
    }
}

/// SetPhysicsState owns a mutable StateChangeData while invoking callbacks.
/// This short-lived adapter borrows the real physical owners, and retains only
/// the pre-transition controller fields needed by LetGo (notably RETURNING).
/// It executes effects immediately; it is not a deferred event queue.
pub(crate) struct Transition<'a, 'board> {
    pub owner: &'a mut Owner,
    pub observation: &'a Observation,
    pub effects: &'a mut Effects<'board>,
    previous: SkateboardControllerFields,
    changed: bool,
}

impl<'a, 'board> Transition<'a, 'board> {
    pub(crate) fn new(
        owner: &'a mut Owner,
        observation: &'a Observation,
        effects: &'a mut Effects<'board>,
        previous: SkateboardControllerFields,
    ) -> Self {
        Self {
            owner,
            observation,
            effects,
            previous,
            changed: false,
        }
    }

    /// Merge only the throw countdown. The caller's state448/system452 writes
    /// must survive. LetGo sets word444=8 for the TU3 throw bit; discarding it
    /// would lose the subsequent eight UpdateFreeBoard torque applications.
    pub(crate) fn finish(self, fields: &mut SkateboardControllerFields) {
        if self.changed {
            fields.word_444 = self.previous.word_444;
        }
    }
}

impl SkateboardControllerActions for Transition<'_, '_> {
    fn hold_skateboard(&mut self) {
        self.previous.word_444 = 0;
        self.owner
            .hold(&mut self.previous, self.observation, self.effects);
        self.changed = true;
    }

    fn let_go_of_skateboard(&mut self) {
        self.previous.word_444 = 0;
        self.owner
            .let_go(&mut self.previous, self.observation, self.effects);
        self.changed = true;
    }
}
