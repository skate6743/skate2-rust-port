//! ScoreModule collector selection82dfc770 and animation smoothing82dfc6e8.
use crate::physics_state_ids::PHYSICS_STATE_HANDPLANT;
use super::{
    super::{scoring::Scoring2, trick::AnimOutTrickData},
    score_module::ScoreModule,
    scoring_attributes::ScoreError,
    scoring_catalog::info,
    scoring_database::TrickDatabase,
};
use crate::physics::FilteredPhysicsState as Filtered;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub(super) enum CollectorKind {
    Invalid = -1,
    Ground = 0,
    Air = 1,
    Grind = 2,
    Wipeout = 3,
    Teleport = 4,
    Offboard = 5,
    Handplant = 6,
}
impl CollectorKind {
    pub(super) fn from_raw(value: i32) -> Result<Self, ScoreError> {
        Ok(match value {
            -1 => Self::Invalid,
            0 => Self::Ground,
            1 => Self::Air,
            2 => Self::Grind,
            3 => Self::Wipeout,
            4 => Self::Teleport,
            5 => Self::Offboard,
            6 => Self::Handplant,
            _ => return Err(ScoreError::Invalid("collector type")),
        })
    }
    pub(super) fn scores_previous(self) -> bool {
        !matches!(self, Self::Wipeout | Self::Teleport | Self::Offboard)
    }
}
impl ScoreModule {
    pub(super) fn select_collector(
        &mut self,
        filtered: Filtered,
        physics: i32,
        requested_teleport: bool,
        grind_scoring_blocked: bool,
        wheels_in_contact: i32,
        animation: &AnimOutTrickData,
        db: &TrickDatabase,
        enabled: &[bool; 14],
        out: &mut Scoring2,
    ) -> Result<CollectorKind, ScoreError> {
        use CollectorKind as K;
        if requested_teleport {
            return Ok(K::Wipeout);
        }
        let state = self
            .reset_suffix
            .as_mut()
            .ok_or(ScoreError::Missing("score reset"))?;
        if ((filtered == Filtered::Ground && wheels_in_contact > 0)
            || matches!(filtered, Filtered::Grind | Filtered::OffBoard))
            && !state.skater_has_been_on_ground
        {
            out.just_reactivated_scoring = true;
            state.skater_has_been_on_ground = true;
        }
        if !state.skater_has_been_on_ground && filtered != Filtered::Wipeout {
            return Ok(K::Teleport);
        }
        if physics == PHYSICS_STATE_HANDPLANT && matches!(filtered, Filtered::Air | Filtered::Ground) {
            return Ok(if enabled[6] { K::Handplant } else { K::Invalid });
        }
        // GetCurrentScorableCategory reads the current name directly, without
        // ProcessAnimData's bit mask or enabled-trick filtering.
        let category = info(db.animation_id(&animation.name)).map(|i| i.category);
        let filtered = match (filtered, state.collector_type, category) {
            (Filtered::Air, 0, Some(0)) => Filtered::Ground,
            (Filtered::Ground, 1, Some(3)) => Filtered::Air,
            _ => filtered,
        };
        Ok(match filtered {
            Filtered::Ground => K::Ground,
            Filtered::Air if enabled[1] => K::Air,
            Filtered::Grind if enabled[5] && !grind_scoring_blocked => K::Grind,
            Filtered::Wipeout => K::Wipeout,
            Filtered::Teleport => K::Teleport,
            Filtered::OffBoard | Filtered::OffBoardAir => K::Offboard,
            _ => K::Invalid,
        })
    }
}
