//! World.UpdateConditioners827af468, selected offline/no-challenge inputs.
//! Geometry comes from completed Update5 outputs and existing entity registries.
use super::{PhysicalAssemblyWorld, PhysicalPlayerAssemblyOwner};
use crate::{
    input_cvar_runtime::scoring::ScoringBool,
    physical_player_phys_out::{
        PhysicalPlayerPhysOut,
        conditioners::scoring_external::{NearbySkater, ScoringExternalInput, TrickPattern},
    },
    players_backend_settings::PlayersBackendSettings,
    skeleton_root_transforms::SkeletonRootVector,
};
use std::sync::Arc;

/// The scoring ids of other online players' skaters: their game slots past the SkaterManager's
/// player indices, so none is taken for a skater the roster lists.
const REMOTE_SKATER_IDS: u32 = 16;

#[derive(Clone, Copy)]
pub(crate) struct ScoringFrameSchedule<'a> {
    pub(crate) tick: u32,
    pub(crate) visual_tick: u32,
    pub(crate) settings: &'a PlayersBackendSettings,
    /// The challenge's popped trick patterns and enabled kinds, the same for every skater.
    pub(crate) challenge: Option<&'a crate::challenge::skate::ScoringInputs>,
}
#[derive(Debug)]
pub(crate) enum ScoringInputError {
    Missing(&'static str),
    Dependency(String),
    NetworkOrGhostPending,
    SpatialDebugPending,
}
impl std::fmt::Display for ScoringInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
fn dependency(e: impl std::fmt::Debug) -> ScoringInputError {
    ScoringInputError::Dependency(format!("{e:?}"))
}
fn need<T>(value: Option<T>, name: &'static str) -> Result<T, ScoringInputError> {
    value.ok_or(ScoringInputError::Missing(name))
}
impl PhysicalAssemblyWorld {
    pub(crate) fn try_scoring_input(
        &self,
        owner: &Arc<PhysicalPlayerAssemblyOwner>,
        output: &PhysicalPlayerPhysOut,
        schedule: ScoringFrameSchedule<'_>,
    ) -> Result<Arc<ScoringExternalInput>, ScoringInputError> {
        use ScoringInputError as E;
        let identity = {
            let query = owner.actor_query.lock().map_err(dependency)?;
            need(query.as_ref(), "Actor query")?
                .try_scoring_identity()
                .map_err(dependency)?
        };
        let (player, flags, non_player_ai) = identity;
        let local = need(self.local_skater_identity.as_ref(), "local manager")?;
        let backend = local.backend();
        if !non_player_ai
            && (backend.try_is_network_game().map_err(dependency)?
                || backend.try_ghost_enabled().map_err(dependency)?)
        {
            return Err(E::NetworkOrGhostPending);
        }
        let cvars = &owner.inputs().cvars;
        let mode = backend.try_game_mode().map_err(dependency)?;
        // Physics::IsHallOfMeatEffectEnabled 0x827acbd0 outside FE: free skate
        // (modes 0/3/4) disables it, then the HOM manager's visuals, then in
        // single player (1, or 7 debug FE) the "Hom Available" setting.
        let (hom_visuals, hom_available) = schedule
            .challenge
            .map_or((true, true), |challenge| (challenge.hom_visuals, challenge.hom_available));
        let hom = cvars
            .try_scoring_bool(ScoringBool::kIsHoMEffectEnabled)
            .map_err(dependency)?
            && (cvars
                .try_scoring_bool(ScoringBool::kIsHoMEffectForceEnabled)
                .map_err(dependency)?
                || (!matches!(mode, 0 | 3 | 4)
                    && hom_visuals
                    && (!matches!(mode, 1 | 7) || hom_available)));
        let mut input = ScoringExternalInput::selected_no_challenge(
            if flags & 2 != 0 {
                schedule.visual_tick
            } else {
                schedule.tick
            },
            schedule.visual_tick,
            hom,
        );
        if let Some(challenge) = schedule.challenge {
            input.activate_patterns = challenge.activate;
            input.deactivate_patterns = challenge.deactivate;
            input.patterns = challenge.patterns.map(|p| TrickPattern::from_words(p.length, p.items));
            input.enabled = challenge.enabled;
            input.multiplier_active = challenge.multiplier_active;
        }
        let row = schedule
            .settings
            .try_pop_scoring(player)
            .map_err(dependency)?;
        input.manuals_are_connectors = row.manuals_are_connectors;
        input.reset_sequence = row.reset_sequence;
        input.reset_scoring = row.reset_scoring;
        input.reset_player_damage = row.reset_damage;
        // CollectNearbyEntitiesForScoring 0x827af180: a non-player AI skater collects nothing.
        if non_player_ai {
            return Ok(Arc::new(input));
        }
        let skeleton = output.try_guarded_skeleton_output().map_err(dependency)?;
        let root = skeleton.skeleton_root_matrix.columns()[3].xyz;
        let category = output.try_guarded_state_output().map_err(dependency)?.physics_state_category;
        let grind_guid = output.try_guarded_grinds_output().map_err(dependency)?.grind_pathed_guid;
        self.grind_services
            .living_world()
            .lock()
            .map_err(dependency)?
            .collect_scoring_entities(root, player, category, grind_guid, &mut input)
            .map_err(dependency)?;
        input.score_over_skaters = true;
        if flags & 1 != 0 && player != 8 {
            // Original uses SkaterManager order, not World allocation order.
            for actor in local.try_scoring_roster().map_err(dependency)? {
                let other_player =
                    need(actor.prefix().map_err(dependency)?.fields(), "roster Actor")?
                        .player_index;
                if other_player == 8 || other_player == player || input.skaters.len() >= 7 {
                    continue;
                }
                let physical = actor.cached_physical_allocation().map_err(dependency)?;
                let other = physical.owner().phys_out().lock().map_err(dependency)?;
                let s = other.try_guarded_skeleton_output().map_err(dependency)?;
                let hips = s.skeleton_hips_pos;
                let dx = skeleton.skeleton_hips_pos.xyz[0] - hips.xyz[0];
                let dz = skeleton.skeleton_hips_pos.xyz[2] - hips.xyz[2];
                // vmsum3fp128: components0+1 then2, Y explicitly zeroed.
                let distance_sq = (dx * dx + 0.0) + dz * dz;
                if !(distance_sq < 9.0) {
                    continue;
                }
                let upper =
                    std::array::from_fn(|i| (s.skeleton_head_pos.xyz[i] + hips.xyz[i]) * 0.5);
                let lower = std::array::from_fn(|i| {
                    ((hips.xyz[i] + hips.xyz[i])
                        + s.skeleton_foot_left_pos.xyz[i]
                        + s.skeleton_foot_right_pos.xyz[i])
                        * 0.25
                });
                input.skaters.push(NearbySkater {
                    upper_body: SkeletonRootVector {
                        xyz: upper,
                        w: None,
                    },
                    lower_body: SkeletonRootVector {
                        xyz: lower,
                        w: None,
                    },
                    unique_id: other_player,
                });
            }
            // The other online players' skaters, which the port does not simulate here, by the
            // same test (PC port: in the original they were the roster's own).
            let remote = self.remote_scoring_skaters.lock().map_err(dependency)?;
            for other in remote.iter() {
                if input.skaters.len() >= 7 {
                    break;
                }
                let dx = skeleton.skeleton_hips_pos.xyz[0] - other.hips[0];
                let dz = skeleton.skeleton_hips_pos.xyz[2] - other.hips[2];
                let distance_sq = (dx * dx + 0.0) + dz * dz;
                if !(distance_sq < 9.0) {
                    continue;
                }
                input.skaters.push(other.nearby(REMOTE_SKATER_IDS + u32::from(other.slot)));
            }
        }
        if cvars
            .try_scoring_bool(ScoringBool::kbDoSpatialScoreTest)
            .map_err(dependency)?
        {
            return Err(E::SpatialDebugPending);
        }
        Ok(Arc::new(input))
    }
}
