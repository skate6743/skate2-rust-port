//! Reached collision and navmesh predicates for an Actor breadcrumb candidate.
//! The caller supplies the retained World, registered ground, and MeshStore;
//! these gates do not create a query owner or infer a successful respawn.

use super::{ActorBreadCrumbManager, ActorBreadCrumbOwnerError, BreadCrumbFloatAttribute};
use crate::{
    attribute_runtime::AttributeRuntimeError,
    collision_line_collection::CollisionLineInput,
    input_cvar_runtime::{InputCvarError, InputCvarReferences},
    living_world_mesh_store::{MeshCell, MeshStore, MeshStoreError},
    physical_player_assembly::PhysicalAssemblyWorld,
    reckoning_numeric::ReckoningNumeric,
    simulation_assembly_backend::constraint_session::physical_world_line::PhysicalWorldLineError,
    simulation_assembly_ground_triangle_topology::AssemblyGroundNumeric,
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
    static_ground_pool::StaticGroundPool,
};

#[derive(Debug)]
pub(crate) enum BreadCrumbCollisionGateError<E> {
    Attribute(AttributeRuntimeError),
    World(PhysicalWorldLineError<E>),
    MissingWorldServices,
    Network(crate::simulation_assembly_backend::constraint_session::world_services::PhysicalWorldServicesError),
}

#[derive(Debug)]
pub(crate) enum BreadCrumbNavMeshGateError {
    Attribute(AttributeRuntimeError),
    Cvar(InputCvarError),
    MissingWorldServices,
    Network(crate::simulation_assembly_backend::constraint_session::world_services::PhysicalWorldServicesError),
    Mesh(MeshStoreError),
    Owner(ActorBreadCrumbOwnerError),
}

impl ActorBreadCrumbManager {
    /// `_TestForCollisionConstraints` 0x82ce19b0, offline reached path.
    /// The Actor island virtual query and Simulation services query both
    /// delegate to the retained World line owner for the selected skater.
    pub(crate) fn try_collision_constraints<N>(
        &self,
        candidate: &SkeletonRootMatrix,
        ignore_capsule_test: bool,
        world: &mut PhysicalAssemblyWorld,
        ground: &StaticGroundPool,
        numeric: &mut N,
    ) -> Result<bool, BreadCrumbCollisionGateError<N::Error>>
    where
        N: AssemblyGroundNumeric + ReckoningNumeric,
    {
        use BreadCrumbCollisionGateError as E;
        let thin_line = breadcrumb_thin_line(candidate.columns()[3]);
        let thin_start = thin_line.start;
        let thin_hit = world
            .try_world_line_test(
                ground,
                thin_line,
                "BreadCrumbManager::_TestForCollisionConstraints",
                numeric,
            )
            .map_err(E::World)?;
        let Some(thin_hit) = thin_hit else {
            return Ok(false);
        };
        if self
            .try_reject_collision_normal(thin_hit.normal.xyz[1])
            .map_err(E::Attribute)?
        {
            return Ok(false);
        }
        if self
            .try_reject_collision_distance(thin_start.xyz[1], thin_hit.position.xyz[1])
            .map_err(E::Attribute)?
        {
            return Ok(false);
        }
        let surface_class = (thin_hit.surface_id >> 7) & 0x1f;
        if !super::breadcrumb_surface_ok(surface_class as u16) {
            return Ok(false);
        }
        if !ignore_capsule_test {
            let height = self
                .try_float_attribute(BreadCrumbFloatAttribute::CapsuleHeightForCollisionTest)
                .map_err(E::Attribute)?;
            let radius = self
                .try_float_attribute(BreadCrumbFloatAttribute::CapsuleRadiusForCollisionTest)
                .map_err(E::Attribute)?;
            // Each endpoint depends on the preceding vector addition.
            if world
                .try_world_line_test(
                    ground,
                    breadcrumb_capsule_line(thin_start, radius, height),
                    "BreadCrumbManager::_TestForCollisionConstraints",
                    numeric,
                )
                .map_err(E::World)?
                .is_some()
            {
                return Ok(false);
            }
        }
        let services = world
            .physical_world_services()
            .ok_or(E::MissingWorldServices)?;
        // The network-game tail (0x82ce1c60): a crumb is kept only well inside the game's
        // invisible boundaries (`TestForNetworkGameCollisionConstraints` with radius -1). With
        // none loaded `IsPositionOk` walks an empty map and its FLT_MAX nearest edge passes.
        if let Some(boundaries) = services.game_info().try_online_boundaries().map_err(E::Network)? {
            return Ok(boundaries.is_position_ok(candidate.columns()[3].xyz, -1.0));
        }
        Ok(true)
    }

    /// `_TestForNavMeshConstraints` 0x82ce14d0, with the original cvar and
    /// network short circuits before MeshStore::GetMeshCell. The selected
    /// MeshStore resolves cells in the currently registered navigation tiles.
    pub(crate) fn try_navmesh_constraints(
        &self,
        candidate: &SkeletonRootMatrix,
        cvars: &InputCvarReferences,
        world: &PhysicalAssemblyWorld,
        mesh: impl FnOnce() -> Result<std::sync::Arc<MeshStore>, ActorBreadCrumbOwnerError>,
    ) -> Result<bool, BreadCrumbNavMeshGateError> {
        use BreadCrumbNavMeshGateError as E;
        if !cvars
            .try_breadcrumb_navmesh_constraints()
            .map_err(E::Cvar)?
        {
            return Ok(true);
        }
        let services = world
            .physical_world_services()
            .ok_or(E::MissingWorldServices)?;
        if services.try_is_network_game().map_err(E::Network)? {
            return Ok(true);
        }
        let mesh = mesh().map_err(E::Owner)?;
        match mesh.try_get_cell(candidate.columns()[3]).map_err(E::Mesh)? {
            MeshCell::Absent => Ok(true),
            cell @ MeshCell::Present { .. } => self
                .try_accept_navmesh_cell_score(cell.score())
                .map_err(E::Attribute),
        }
    }
}

// Raw VMX128 operands at 0x82ce1a40/44 both read v63: the second
// addition consumes the already raised start, not the candidate position.
fn breadcrumb_thin_line(position: SkeletonRootVector) -> CollisionLineInput {
    let start = position.zip(SkeletonRootVector::known([0.0, 0.1, 0.0, 0.0]), |a, b| {
        a + b
    });
    let end = start.zip(SkeletonRootVector::known([0.0, -10.0, 0.0, 0.0]), |a, b| {
        a + b
    });
    CollisionLineInput {
        start,
        end,
        fatness: 0.0,
    }
}

// 0x82ce1c0c writes v63; 0x82ce1c14 then reads that same v63.
fn breadcrumb_capsule_line(
    thin_start: SkeletonRootVector,
    radius: f32,
    height: f32,
) -> CollisionLineInput {
    let start = thin_start.zip(
        SkeletonRootVector::known([0.0, radius, 0.0, 0.0]),
        |a, b| a + b,
    );
    let end = start.zip(
        SkeletonRootVector::known([0.0, height, 0.0, 0.0]),
        |a, b| a + b,
    );
    CollisionLineInput {
        start,
        end,
        fatness: radius,
    }
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;

    #[test]
    fn collision_endpoints_follow_native_dependent_additions() {
        let thin = breadcrumb_thin_line(SkeletonRootVector::known([3.0, 4.0, 5.0, 7.0]));
        assert_eq!(thin.start.known_lanes(), Some([3.0, 4.1, 5.0, 7.0]));
        assert_eq!(
            thin.end.known_lanes(),
            Some([3.0, 4.1_f32 - 10.0, 5.0, 7.0])
        );
        assert_eq!(thin.fatness, 0.0);
        let capsule = breadcrumb_capsule_line(thin.start, 0.5, 1.5);
        assert_eq!(capsule.start.known_lanes(), Some([3.0, 4.6, 5.0, 7.0]));
        assert_eq!(capsule.end.known_lanes(), Some([3.0, 6.1, 5.0, 7.0]));
        assert_eq!(capsule.fatness, 0.5);
    }

    #[test]
    fn endpoint_additions_keep_unknown_w_and_round_each_step() {
        let thin = breadcrumb_thin_line(SkeletonRootVector {
            xyz: [1.0, 2097152.0, 3.0],
            w: None,
        });
        // +0.1 rounds away at this magnitude. Folding the two offsets
        // into -9.9 would yield a different endpoint.
        assert_eq!(thin.end.xyz[1], 2097142.0);
        assert_eq!(thin.start.w, None);
        assert_eq!(thin.end.w, None);
        let capsule = breadcrumb_capsule_line(thin.start, 0.125, 0.125);
        assert_eq!(capsule.end.xyz[1], 2097152.0);
        assert_eq!(capsule.start.w, None);
        assert_eq!(capsule.end.w, None);
    }
}
