//! Actor-owned skater breadcrumb storage, separate from camera breadcrumbs.
//! Source: BreadCrumbManager::Implementation 0x82ce33b8, DropBreadCrumb
//! 0x82ce1fd0, ring push_back 0x82ce1e30, and selected pop 0x82ce2d80.
//! Collision, navmesh, and living-world answers must come from their owners.
use crate::{
    actor_construction::{ActorConstruction, ActorConstructionError, ActorPrefixPhase},
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    skeleton_root_transforms::{SkeletonRootMatrix, SkeletonRootVector},
};
use std::sync::{Arc, Mutex};

#[path = "actor_breadcrumb_constructor_owners.rs"]
mod constructor_owners;
pub(crate) use constructor_owners::{
    ActorBreadCrumbConstructorOwners, ActorBreadCrumbOwnerError, VerifiedBreadCrumbOwners,
};

#[path = "actor_breadcrumb_gate_attributes.rs"]
mod gate_attributes;
pub(crate) use gate_attributes::{BreadCrumbFloatAttribute, breadcrumb_surface_ok};
#[path = "actor_breadcrumb_collision_navmesh.rs"]
mod collision_navmesh;
pub(crate) use collision_navmesh::{BreadCrumbCollisionGateError, BreadCrumbNavMeshGateError};
#[path = "actor_breadcrumb_live_selection.rs"]
mod live_selection;
pub(crate) use live_selection::{LiveBreadCrumbSelectionError, LiveBreadCrumbSelectionSource};
#[path = "actor_breadcrumb_calc_respawn_transform.rs"]
pub(crate) mod calc_respawn_transform;

pub(crate) const CLASS: u64 = 0x12b6_4c0e_804b_0853;
const DEFAULT_COLLECTION: u64 = 0xd7ed_bd36_2d7d_2152;
const RING_STORAGE: usize = 33;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SkaterBreadCrumb {
    pub(crate) transform: SkeletonRootMatrix,
    /// `Actor::GetFootForwardWithFakie` returns the RIGHT enum as zero.
    pub(crate) foot_forward_right: bool,
    pub(crate) offboard: bool,
}

/// Host indices represent the native `mBegin`, `mEnd`, and 33-element vector.
/// Native push keeps one element unused, so at most 32 breadcrumbs are live.
pub(crate) struct ActorBreadCrumbManager {
    pub(crate) spawn_position: SkeletonRootMatrix,
    pub(crate) override_respawn: SkaterBreadCrumb,
    pub(crate) override_active: bool,
    pub(crate) last_known_orientation: SkeletonRootMatrix,
    pub(crate) last_known_actor_position: SkeletonRootVector,
    pub(crate) attributes: AttributeRuntimeInstance,
    pub(crate) constructor_owners: ActorBreadCrumbConstructorOwners,
    /// PC: counts the managers a new Actor would have had (`respawn`), so a breadcrumb history
    /// kept by another physics owner of the same Actor restarts with them.
    pub(crate) spawn_generation: u64,
    ring: [Option<SkaterBreadCrumb>; RING_STORAGE],
    begin: usize,
    end: usize,
    size: usize,
}

impl ActorBreadCrumbManager {
    /// Native Actor+0x204 allocation follows the physical constructor return.
    /// The caller supplies the live `GetFootForwardWithFakie` result sampled by
    /// DropBreadCrumb; no stance or fakie value is inferred from the spawn.
    pub(crate) fn new(
        spawn: SkeletonRootMatrix,
        foot_forward_right: bool,
        vault: &Arc<AttributeRuntimeVault>,
        constructor_owners: ActorBreadCrumbConstructorOwners,
    ) -> Self {
        let mut manager = Self {
            spawn_position: spawn,
            override_respawn: SkaterBreadCrumb {
                transform: spawn,
                foot_forward_right: true,
                offboard: false,
            },
            override_active: false,
            last_known_orientation: SkeletonRootMatrix::known([
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ]),
            last_known_actor_position: spawn.columns()[3],
            attributes: vault.instance(CLASS, DEFAULT_COLLECTION),
            constructor_owners,
            spawn_generation: 0,
            ring: [None; RING_STORAGE],
            begin: 0,
            end: 0,
            size: 0,
        };
        manager.drop_breadcrumb(spawn, foot_forward_right, false);
        manager
    }

    /// The manager a new Actor would have at `spawn`, for a skater the port keeps where
    /// `SkaterManager::OnCommand` case 3 (`0x82600920`) destroys it and `SpawnPlayer`
    /// (`0x82600630`) makes another: the constructor's (`0x82ce33b8`) spawn position, override
    /// (unset, RIGHT, on the board), identity last known orientation (w zero) and last known
    /// position, and an empty ring that `DropBreadCrumb` (`0x82ce1fd0`) gives the spawn crumb.
    pub(crate) fn respawn(&mut self, spawn: SkeletonRootMatrix, foot_forward_right: bool) {
        self.spawn_position = spawn;
        self.override_respawn = SkaterBreadCrumb { transform: spawn, foot_forward_right: true, offboard: false };
        self.override_active = false;
        self.last_known_orientation = SkeletonRootMatrix::known([
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ]);
        self.last_known_actor_position = spawn.columns()[3];
        self.ring = [None; RING_STORAGE];
        self.begin = 0;
        self.end = 0;
        self.size = 0;
        self.drop_breadcrumb(spawn, foot_forward_right, false);
        self.spawn_generation = self.spawn_generation.wrapping_add(1);
    }

    /// The live ring, oldest first.
    pub(crate) fn ring_oldest_first(&self) -> Vec<SkaterBreadCrumb> {
        (0..self.size)
            .filter_map(|age| self.ring[(self.begin + age) % RING_STORAGE])
            .collect()
    }

    /// Skate 3 physics records the Actor's crumbs with TU3's breadcrumb Update (82BFB1E0) and
    /// selects them with its own selectors; this manager shows that history to the rest of the
    /// game: its entries, riding orientation and current position.
    pub(crate) fn mirror(
        &mut self,
        entries: impl IntoIterator<Item = SkaterBreadCrumb>,
        orientation: SkeletonRootMatrix,
        position: SkeletonRootVector,
    ) {
        self.ring = [None; RING_STORAGE];
        self.begin = 0;
        self.end = 0;
        self.size = 0;
        for crumb in entries {
            self.drop_breadcrumb(crumb.transform, crumb.foot_forward_right, crumb.offboard);
        }
        self.last_known_orientation = orientation;
        self.last_known_actor_position = position;
    }

    pub(crate) fn len(&self) -> usize {
        self.size
    }

    /// `Implementation::Update` reads the last ring entry only when nonempty.
    pub(crate) fn newest(&self) -> Option<SkaterBreadCrumb> {
        if self.size == 0 {
            None
        } else {
            let index = if self.end == 0 {
                RING_STORAGE - 1
            } else {
                self.end - 1
            };
            self.ring[index]
        }
    }

    /// Upgrade the native constructor's non-owning service references before
    /// acquiring the World for a reached collision/navmesh/LW query.
    pub(crate) fn try_query_owners(
        &self,
    ) -> Result<VerifiedBreadCrumbOwners, ActorBreadCrumbOwnerError> {
        self.constructor_owners.try_verify()
    }

    /// Same identity check when the selected pre-solve already owns World.
    pub(crate) fn try_query_owners_in_world(
        &self,
        simulation: &Arc<Mutex<crate::physical_player_assembly::PhysicalAssemblyWorld>>,
        world: &crate::physical_player_assembly::PhysicalAssemblyWorld,
    ) -> Result<VerifiedBreadCrumbOwners, ActorBreadCrumbOwnerError> {
        self.constructor_owners
            .try_verify_in_world(simulation, world)
    }

    /// Native DropBreadCrumb copies all matrix lanes, records foot/fakie and
    /// offboard, then calls the 33-slot EASTL ring's push_back.
    pub(crate) fn drop_breadcrumb(
        &mut self,
        transform: SkeletonRootMatrix,
        foot_forward_right: bool,
        offboard: bool,
    ) {
        self.ring[self.end] = Some(SkaterBreadCrumb {
            transform,
            foot_forward_right,
            offboard,
        });
        self.end = (self.end + 1) % RING_STORAGE;
        if self.end == self.begin {
            self.begin = (self.begin + 1) % RING_STORAGE;
        } else {
            self.size += 1;
        }
    }

    /// Native PopBreadCrumb scans newest to oldest. A rejected newer crumb is
    /// retained. `ring_buffer::erase` copies subsequent entries toward the
    /// selected slot, then decrements the end pointer.
    pub(crate) fn try_pop_valid<E>(
        &mut self,
        mut living_world_accepts: impl FnMut(&SkaterBreadCrumb) -> Result<bool, E>,
    ) -> Result<Option<SkaterBreadCrumb>, E> {
        let mut index = self.end;
        for _ in 0..self.size {
            index = if index == 0 {
                RING_STORAGE - 1
            } else {
                index - 1
            };
            let breadcrumb = self.ring[index].expect("native live ring entry");
            if living_world_accepts(&breadcrumb)? {
                let mut cursor = index;
                let last = if self.end == 0 {
                    RING_STORAGE - 1
                } else {
                    self.end - 1
                };
                while cursor != last {
                    let next = (cursor + 1) % RING_STORAGE;
                    self.ring[cursor] = self.ring[next];
                    cursor = next;
                }
                self.ring[last] = None;
                self.end = last;
                self.size -= 1;
                return Ok(Some(breadcrumb));
            }
        }
        Ok(None)
    }

    /// GetSuggestedBreadCrumb 0x82ce3060 delegates to the override or to
    /// PopSuggestedRespawnPosition 0x82ce2ef8. The source must answer every
    /// collision, navmesh, and living-world gate before a reset is published.
    pub(crate) fn try_suggest<S: BreadCrumbSelectionSource>(
        &mut self,
        source: &mut S,
    ) -> Result<SkaterBreadCrumb, S::Error> {
        if self.override_active {
            self.override_active = false;
            return Ok(self.override_respawn);
        }
        if let Some(skate3) = source.try_skate3_selection(true)? {
            return Ok(skate3);
        }
        if let Some(wipeout_spot) = source.try_wipeout_spot(self)? {
            return Ok(wipeout_spot);
        }
        source.try_begin_ring(self)?;
        if let Some(ring) = self.try_pop_valid(|crumb| source.try_living_world_accepts(crumb))? {
            return Ok(ring);
        }
        source.try_last_resort(self)
    }

    /// GetSuggestedRespawnPosition 0x82ce2698, the const query: no override,
    /// and GetBreadCrumb 0x82ce2428 scans newest to oldest without erasing
    /// the crumb the living world accepts; otherwise `try_suggest`'s order.
    pub(crate) fn try_suggested_respawn<S: BreadCrumbSelectionSource>(
        &self,
        source: &mut S,
    ) -> Result<SkaterBreadCrumb, S::Error> {
        if let Some(skate3) = source.try_skate3_selection(false)? {
            return Ok(skate3);
        }
        if let Some(wipeout_spot) = source.try_wipeout_spot(self)? {
            return Ok(wipeout_spot);
        }
        source.try_begin_ring(self)?;
        let mut index = self.end;
        for _ in 0..self.size {
            index = if index == 0 {
                RING_STORAGE - 1
            } else {
                index - 1
            };
            let breadcrumb = self.ring[index].expect("native live ring entry");
            if source.try_living_world_accepts(&breadcrumb)? {
                return Ok(breadcrumb);
            }
        }
        source.try_last_resort(self)
    }
}

/// Exact native selector branch order with still-unmounted gate owners. The
/// implementation must call the reached original collision/navmesh/LW queries;
/// this interface cannot silently assume the spawn is a valid reset location.
pub(crate) trait BreadCrumbSelectionSource {
    type Error;
    /// With Skate 3 physics the Actor's crumbs are TU3's: PopSuggestedRespawnPosition 82BFC9B0
    /// (`pop`) or GetSuggestedRespawnPosition 82BFC828 over that history. None otherwise.
    fn try_skate3_selection(&mut self, _pop: bool) -> Result<Option<SkaterBreadCrumb>, Self::Error> {
        Ok(None)
    }
    fn try_wipeout_spot(
        &mut self,
        breadcrumbs: &ActorBreadCrumbManager,
    ) -> Result<Option<SkaterBreadCrumb>, Self::Error>;
    /// PopBreadCrumb reads both LW sphere radii before its empty-ring check.
    fn try_begin_ring(&mut self, breadcrumbs: &ActorBreadCrumbManager) -> Result<(), Self::Error>;
    fn try_living_world_accepts(
        &mut self,
        breadcrumb: &SkaterBreadCrumb,
    ) -> Result<bool, Self::Error>;
    fn try_last_resort(
        &mut self,
        breadcrumbs: &ActorBreadCrumbManager,
    ) -> Result<SkaterBreadCrumb, Self::Error>;
}

#[derive(Debug)]
pub(crate) enum ActorBreadCrumbMountError {
    Actor(ActorConstructionError),
    MissingPhysicalReturn,
    MissingAnimation,
    MissingAnimationPlayer,
    UnknownRidingFakie,
    AnimationLock,
    Owners(ActorBreadCrumbOwnerError),
    AlreadyConstructed,
}

#[derive(Debug)]
pub(crate) enum ActorAppropriateTeleportError<E> {
    Actor(ActorConstructionError),
    MissingPhysicsAuxiliary,
    Ai(String),
    MissingBreadcrumbs,
    ForeignBreadcrumbs,
    BreadcrumbLock,
    Select(E),
    Teleport(crate::actor_construction::teleport::ActorTeleportError),
}

/// The live AI owner answers this at the native call site, after animation
/// and any earlier skater teleports. None selects the ordinary breadcrumb
/// path (including an ambient skater whose unspawn was just queued).
pub(crate) type ActorAiRespawnQuery<'a> = dyn FnMut(&ActorConstruction) -> Result<Option<SkeletonRootMatrix>, String> + 'a;

impl ActorConstruction {
    /// SerializeBreadCrumbs82ce30f8 copies newest first, up to the 32 live slots.
    pub(crate) fn try_service_breadcrumbs(&self)->Result<Option<(Vec<SkaterBreadCrumb>,AttributeRuntimeInstance)>,String>{
        let state=self.prefix().map_err(|e|e.to_string())?;
        let Some(owner)=state.breadcrumbs_owner.as_ref().cloned() else{return Ok(None);};
        drop(state);
        let crumbs=owner.lock().map_err(|_|"service breadcrumb lock")?;
        let mut result=Vec::with_capacity(crumbs.size);let mut index=crumbs.end;
        for _ in 0..crumbs.size{
            index=if index==0{RING_STORAGE-1}else{index-1};
            result.push(crumbs.ring[index].ok_or("native live breadcrumb absent")?);
        }
        Ok(Some((result,crumbs.attributes.clone())))
    }
    /// `Actor::TeleportSomewhereAppropriate` 0x825f0e18. The AI owner handles
    /// AC_AMBIENT/PT_AI_Ambient unspawn or GetAppropriateRespawnTransform824cc208.
    /// A successful AI query teleports on-board without changing spawn stance;
    /// otherwise the original breadcrumb/stance/off-board path follows.
    pub(crate) fn try_teleport_somewhere_appropriate<S: BreadCrumbSelectionSource>(
        &self,
        breadcrumbs_owner: &Arc<Mutex<ActorBreadCrumbManager>>,
        source: &mut S,
        ai_respawn: &mut ActorAiRespawnQuery<'_>,
    ) -> Result<(), ActorAppropriateTeleportError<S::Error>> {
        use ActorAppropriateTeleportError as E;
        let state = self.prefix().map_err(E::Actor)?;
        let has_ai = state
            .physics_auxiliary
            .as_ref()
            .ok_or(E::MissingPhysicsAuxiliary)?
            .ai_controller.is_some();
        drop(state);
        if has_ai && let Some(transform) = ai_respawn(self).map_err(E::Ai)? {
            return self.try_publish_teleport(&transform, true).map_err(E::Teleport);
        }
        let state = self.prefix().map_err(E::Actor)?;
        let retained = state
            .breadcrumbs_owner
            .as_ref()
            .ok_or(E::MissingBreadcrumbs)?;
        if !Arc::ptr_eq(retained, breadcrumbs_owner) {
            return Err(E::ForeignBreadcrumbs);
        }
        drop(state);

        // Actor's virtual +0x1c returns this same manager. The selector owns
        // override/wipeout/ring/last-resort order and may erase one ring entry.
        let selected = breadcrumbs_owner
            .lock()
            .map_err(|_| E::BreadcrumbLock)?
            .try_suggest(source)
            .map_err(E::Select)?;

        // 0x825f0ecc..ef0 inline Actor::SpawnFootForwardStance with the
        // crumb's stance: Actor+0x1fc minus 0x52e0 is the SkaterAnim object,
        // whose own vtable +0xc0 is SetSpawnFootForwardStance 0x82cb7868
        // (ISkaterAnim +0xc0 would be IsTryingToSprint). +0x240 follows.
        self.try_spawn_foot_forward_stance(u32::from(!selected.foot_forward_right))
            .map_err(E::Teleport)?;
        // Preserve each source column's W knowledge through the native reset
        // copy. A missing producer remains typed unknown through PhysOut.
        self.try_publish_teleport(&selected.transform, !selected.offboard)
            .map_err(E::Teleport)
    }

    /// `Actor::GetFootForwardWithFakie` 0x825ed560 calls the published
    /// SkaterAnim's `GetFootForward` 0x82cb78a0, then flips its zero/one
    /// answer when `GetIsRidingFakie` 0x82cb7530 is true. The first getter
    /// reads natural and relative stance at SkaterAnim+0x6138/+0x6140.
    pub(crate) fn try_construct_breadcrumbs_from_animation(
        &self,
        vault: &Arc<AttributeRuntimeVault>,
        constructor_owners: ActorBreadCrumbConstructorOwners,
    ) -> Result<Arc<Mutex<ActorBreadCrumbManager>>, ActorBreadCrumbMountError> {
        let foot_forward_right = self.try_current_foot_forward_with_fakie()?;
        self.try_construct_breadcrumbs(foot_forward_right, vault, constructor_owners)
    }

    /// Re-sample the same published animation when a successful wipeout spot
    /// asks Actor::GetFootForwardWithFakie, rather than reusing spawn stance.
    pub(crate) fn try_current_foot_forward_with_fakie(
        &self,
    ) -> Result<bool, ActorBreadCrumbMountError> {
        use ActorBreadCrumbMountError as E;
        let animation = self
            .published_camera_animation()
            .map_err(E::Actor)?
            .ok_or(E::MissingAnimation)?;
        let animation = animation.lock().map_err(|_| E::AnimationLock)?;
        #[cfg(feature = "skate3-runtime")]
        if let Some(right) = animation.mode_foot_forward_right {
            return Ok(right);
        }
        let player = animation.player().ok_or(E::MissingAnimationPlayer)?;
        if player.flags_known_mask & 0x2000_0000 == 0 {
            return Err(E::UnknownRidingFakie);
        }
        let forward_left = animation.try_foot_forward().map_err(|_| E::MissingAnimationPlayer)? == 1;
        let riding_fakie = player.flags_bits & 0x2000_0000 != 0;
        let foot_forward_right = !(forward_left ^ riding_fakie);
        Ok(foot_forward_right)
    }

    /// Actor constructor 0x825ff300..444: physical publication precedes the
    /// breadcrumb allocation and the later physics auxiliaries. The caller
    /// supplies the live animation's foot-forward-with-fakie result.
    pub(crate) fn try_construct_breadcrumbs(
        &self,
        foot_forward_right: bool,
        vault: &Arc<AttributeRuntimeVault>,
        constructor_owners: ActorBreadCrumbConstructorOwners,
    ) -> Result<Arc<Mutex<ActorBreadCrumbManager>>, ActorBreadCrumbMountError> {
        use ActorBreadCrumbMountError as E;
        let mut state = self.prefix().map_err(E::Actor)?;
        if state.phase != ActorPrefixPhase::PhysicalConstructorReturned
            || state.physical_cached.is_none()
        {
            return Err(E::MissingPhysicalReturn);
        }
        if !state.animation_published {
            return Err(E::MissingAnimation);
        }
        if state.breadcrumbs_owner.is_some() {
            return Err(E::AlreadyConstructed);
        }
        let manager = Arc::new(Mutex::new(ActorBreadCrumbManager::new(
            self.spawn,
            foot_forward_right,
            vault,
            constructor_owners,
        )));
        state.breadcrumbs_owner = Some(Arc::clone(&manager));
        state
            .fields
            .as_mut()
            .expect("Actor fields precede physical")
            .breadcrumbs_null = false;
        Ok(manager)
    }

    pub(crate) fn retained_breadcrumbs(
        &self,
    ) -> Result<Option<Arc<Mutex<ActorBreadCrumbManager>>>, ActorConstructionError> {
        Ok(self.prefix()?.breadcrumbs_owner.as_ref().map(Arc::clone))
    }
}

#[cfg(test)]
mod suggestion_tests {
    use super::*;

    /// No wipeout spot; the living world accepts only the crumb at x; the
    /// newest crumb is the last resort.
    struct Accepting(f32);

    impl BreadCrumbSelectionSource for Accepting {
        type Error = ();
        fn try_wipeout_spot(&mut self, _: &ActorBreadCrumbManager) -> Result<Option<SkaterBreadCrumb>, ()> {
            Ok(None)
        }
        fn try_begin_ring(&mut self, _: &ActorBreadCrumbManager) -> Result<(), ()> {
            Ok(())
        }
        fn try_living_world_accepts(&mut self, crumb: &SkaterBreadCrumb) -> Result<bool, ()> {
            Ok(crumb.transform.columns()[3].xyz[0] == self.0)
        }
        fn try_last_resort(&mut self, breadcrumbs: &ActorBreadCrumbManager) -> Result<SkaterBreadCrumb, ()> {
            breadcrumbs.newest().ok_or(())
        }
    }

    fn at(x: f32) -> SkeletonRootMatrix {
        SkeletonRootMatrix::known([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, 0.0, 0.0, 1.0])
    }

    #[test]
    #[ignore = "requires original SKATE2_GAME_ROOT assets"]
    fn the_suggested_respawn_keeps_the_crumb_the_pop_takes() {
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("game root"));
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES {
            let path = spec.path_under(&root);
            if path.is_file() {
                mounts.push(MountedBig::open(path, spec.use_external_header).unwrap());
            }
        }
        let vault = Arc::new(
            AttributeRuntimeVault::try_load_mounted(&mounts, "data/db/skater.vlt", &crate::selected_original_asset_classes::selected_classes())
                .unwrap(),
        );
        let mut crumbs = ActorBreadCrumbManager::new(at(0.0), true, &vault, ActorBreadCrumbConstructorOwners::detached());
        crumbs.drop_breadcrumb(at(1.0), true, false);
        crumbs.drop_breadcrumb(at(2.0), true, false);
        // GetBreadCrumb takes the newest accepted crumb and keeps it.
        assert_eq!(crumbs.try_suggested_respawn(&mut Accepting(1.0)).unwrap().transform, at(1.0));
        assert_eq!(crumbs.len(), 3);
        // Nothing accepted: the last resort.
        assert_eq!(crumbs.try_suggested_respawn(&mut Accepting(9.0)).unwrap().transform, at(2.0));
        // PopBreadCrumb erases what it takes.
        assert_eq!(crumbs.try_suggest(&mut Accepting(1.0)).unwrap().transform, at(1.0));
        assert_eq!(crumbs.len(), 2);
    }
}
