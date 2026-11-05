//! SkaterManager::PostUpdate 0x825f8ba4..8c08 and breadcrumb Update82ce2230.
use crate::physics_state_ids::{
    PHYSICS_STATE_OFFBOARD, PHYSICS_STATE_OFFBOARD_LOCOMOTION, PHYSICS_STATE_OFFBOARD_PUSHING,
    PHYSICS_STATE_PHYSICS_GROUND, PHYSICS_STATE_SKITCHING,
};
use super::{
    ActorConstruction, ActorConstructionError,
    breadcrumbs::{
        ActorBreadCrumbMountError, ActorBreadCrumbOwnerError, BreadCrumbCollisionGateError,
        BreadCrumbFloatAttribute, BreadCrumbNavMeshGateError, breadcrumb_surface_ok,
        calc_respawn_transform::{CalcRespawnTransformError, try_calc_respawn_transform},
    },
    physical_mount::ActorPhysicalMountError,
};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    collision_line_numeric::HostCollisionLineNumeric,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    physical_player_assembly::PhysicalAssemblyWorld,
    physical_player_phys_out::construct::ConcretePhysOutError,
    reckoning_numeric::{HostReckoningNumeric, ReckoningHostNumericError, ReckoningNumeric},
    static_ground_pool::StaticGroundPool,
};

use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) enum ActorBreadCrumbPostUpdateError {
    Actor(ActorConstructionError),
    MissingBreadcrumbs,
    Physical(ActorPhysicalMountError),
    OutputLock,
    Skeleton(ConcretePhysOutError<std::convert::Infallible>),
    State(ConcretePhysOutError<std::convert::Infallible>),
    BreadcrumbLock,
    Cvar(InputCvarError),
    Attribute(AttributeRuntimeError),
    Numeric(ReckoningHostNumericError),
    Calc(CalcRespawnTransformError<ReckoningHostNumericError>),
    Owner(ActorBreadCrumbOwnerError),
    Output(ConcretePhysOutError<std::convert::Infallible>),
    Collision(BreadCrumbCollisionGateError<ReckoningHostNumericError>),
    NavMesh(BreadCrumbNavMeshGateError),
    FootForward(ActorBreadCrumbMountError),
    Skate3Lock,
}

impl ActorConstruction {
    /// The manager's roster loop tests Actor flags against `updateMask`, then
    /// excludes AI-controlled actors. Its reached breadcrumb Update obtains
    /// this Actor's `PhysOut_Skeleton` and copies `mSkeletonHipsPos` as a full
    /// 16-byte vector before testing frames since reset. If the outer gates
    /// reach Ground100/104, the same manager receives CalcRespawnTransform's
    /// full orientation before the later physics constraints are attempted.
    pub(crate) fn try_post_update_breadcrumb_hips(
        &self,
        update_mask: u32,
        cvars: &InputCvarReferences,
        numeric: &mut HostReckoningNumeric,
        simulation: &Arc<Mutex<PhysicalAssemblyWorld>>,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        line_numeric: &mut HostCollisionLineNumeric,
    ) -> Result<bool, ActorBreadCrumbPostUpdateError> {
        use ActorBreadCrumbPostUpdateError as E;
        let state = self.prefix().map_err(E::Actor)?;
        let flags = state
            .fields()
            .ok_or(E::Actor(ActorConstructionError::ActorFieldsUninitialized))?
            .flags;
        if flags & update_mask == 0
            || state
                .physics_auxiliary
                .as_ref()
                .is_some_and(|auxiliary| auxiliary.ai_controller.is_some())
        {
            return Ok(false);
        }
        let breadcrumbs = state
            .breadcrumbs_owner
            .as_ref()
            .map(std::sync::Arc::clone)
            .ok_or(E::MissingBreadcrumbs)?;
        drop(state);

        let physical = self.cached_physical_allocation().map_err(E::Physical)?;
        // With Skate 3 physics, TU3's breadcrumb Update (82BFB1E0) runs after the source tick
        // and this manager mirrors its history.
        #[cfg(feature = "skate3-runtime")]
        if physical.owner().skate3.lock().map_err(|_| E::Skate3Lock)?.is_some() {
            return Ok(false);
        }
        let output = physical
            .owner()
            .phys_out()
            .lock()
            .map_err(|_| E::OutputLock)?;
        let hips = output
            .try_skeleton_output()
            .map_err(E::Skeleton)?
            .skeleton_hips_pos;
        let mut breadcrumbs = breadcrumbs.lock().map_err(|_| E::BreadcrumbLock)?;
        breadcrumbs.last_known_actor_position = hips;

        // Update 0x82ce22c8..2300: signed >=; the ring distance test is
        // absent for an empty ring. Hips remain committed on every exit.
        let state = output.try_state_output().map_err(E::State)?;
        if state.frames_since_reset
            < cvars
                .try_breadcrumb_min_ticks_after_reset()
                .map_err(E::Cvar)?
        {
            return Ok(true);
        }
        if let Some(newest) = breadcrumbs.newest() {
            let current_root = output
                .try_skeleton_output()
                .map_err(E::Skeleton)?
                .skeleton_root_matrix
                .columns()[3];
            let minimum = breadcrumbs
                .try_float_attribute(BreadCrumbFloatAttribute::UpdateMinimumDistance)
                .map_err(E::Attribute)?;
            let delta = newest.transform.columns()[3].zip(current_root, |a, b| a - b);
            let distance_square = numeric.dot_vector(delta, delta).map_err(E::Numeric)?;
            if minimum * minimum > distance_square {
                return Ok(true);
            }
        }

        // _TestForPhysicsConstraints 0x82ce189c..190c. Its orientation copy
        // precedes state-count, hang-primitive and surface rejection.
        if state.physics_requested_teleport {
            return Ok(true);
        }
        match state.physics_state {
            PHYSICS_STATE_PHYSICS_GROUND | PHYSICS_STATE_SKITCHING => {
                let orientation =
                    try_calc_respawn_transform(&output, cvars, numeric).map_err(E::Calc)?;
                breadcrumbs.last_known_orientation = orientation;
                let minimum = breadcrumbs.try_min_ticks_in_state().map_err(E::Attribute)?;
                if !try_ground_constraints(
                    state.state_update_count,
                    minimum,
                    || {
                        output
                            .try_camera_grinds_output()
                            .map(|grinds| grinds.hang_primitive_exists)
                            .map_err(E::Output)
                    },
                    || output.try_breadcrumb_collision_surface().map_err(E::Output),
                )? {
                    return Ok(true);
                }
            }
            PHYSICS_STATE_OFFBOARD | PHYSICS_STATE_OFFBOARD_PUSHING | PHYSICS_STATE_OFFBOARD_LOCOMOTION => {
                if state.state_update_count
                    <= breadcrumbs.try_min_ticks_in_state().map_err(E::Attribute)?
                {
                    return Ok(true);
                }
            }
            _ => return Ok(true),
        }
        // Native Update recomputes, rather than reusing the orientation commit.
        let candidate = try_calc_respawn_transform(&output, cvars, numeric).map_err(E::Calc)?;
        let ignore_capsule = state.physics_state == PHYSICS_STATE_SKITCHING;
        let offboard = state.is_offboard;
        drop(output);
        breadcrumbs
            .constructor_owners
            .try_verify_world(simulation)
            .map_err(E::Owner)?;
        if !breadcrumbs
            .try_collision_constraints(&candidate, ignore_capsule, world, ground, line_numeric)
            .map_err(E::Collision)?
        {
            return Ok(true);
        }
        if !breadcrumbs
            .try_navmesh_constraints(&candidate, cvars, world, || {
                breadcrumbs
                    .constructor_owners
                    .try_verify_in_world(simulation, world)
                    .map(|owners| owners.mesh_store)
            })
            .map_err(E::NavMesh)?
        {
            return Ok(true);
        }
        let foot_forward = self
            .try_current_foot_forward_with_fakie()
            .map_err(E::FootForward)?;
        breadcrumbs.drop_breadcrumb(candidate, foot_forward, offboard);
        Ok(true)
    }
}

/// Preserve native lazy count -> hang -> ushort surface reads.
fn try_ground_constraints<E>(
    count: i32,
    minimum: i32,
    hang: impl FnOnce() -> Result<bool, E>,
    surface: impl FnOnce() -> Result<u32, E>,
) -> Result<bool, E> {
    if count <= minimum || hang()? {
        return Ok(false);
    }
    Ok(breadcrumb_surface_ok(surface()? as u16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_rejection_does_not_read_later_outputs() {
        assert!(
            !try_ground_constraints::<()>(
                12,
                12,
                || panic!("hang before count"),
                || panic!("surface before count")
            )
            .unwrap()
        );
        assert!(
            !try_ground_constraints::<()>(
                13,
                12,
                || Ok(true),
                || panic!("surface after hang rejection")
            )
            .unwrap()
        );
        assert!(try_ground_constraints::<()>(-1, -2, || Ok(false), || Ok(0)).unwrap());
        assert!(
            try_ground_constraints(13, 12, || Err("hang"), || panic!("surface after error"))
                .is_err()
        );
    }

    #[test]
    fn ground_surface_is_low_ushort_enum_not_packed_hit_class() {
        for surface in 0..=u16::MAX {
            assert_eq!(
                try_ground_constraints::<()>(
                    1,
                    0,
                    || Ok(false),
                    || Ok(0xabcd_0000 | u32::from(surface))
                )
                .unwrap(),
                ![5, 6, 8, 9].contains(&surface)
            );
        }
    }
}
