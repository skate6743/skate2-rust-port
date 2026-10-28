//! Retained selected `GameInfo` and World physical collection constructor fields.
//! This does not construct the other Simulation or World subsystems.
use crate::{
    actor_construction::physical_mount::ActorPhysicalAllocation,
    simulation_contact_jacobian::network_preparation::SimulationManagerContactAuthority,
};
use std::sync::{Arc, Mutex, Weak};

#[derive(Debug)]
pub(crate) enum SelectedMainWorldError {
    Backend(crate::skater_manager_local_identity::LocalIdentityError),
    Construction(super::SimulationAssemblyConstraintInitError),
}
impl std::fmt::Display for SelectedMainWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Backend(e) => write!(f, "selected world backend: {e}"),
            Self::Construction(e) => write!(f, "selected main simulation: {e}"),
        }
    }
}

impl<M, C> super::SimulationAssemblyConstraintSession<M, C> {
    /// Main simulation projection of `World::Initialize827b0e58`: capture the
    /// actual backend ghost byte at0f38 before constructing manager0fc0. Main
    /// Params provide860 bodies (including static),360 joints,540 drives,2040
    /// contacts. This does not initialize the separate visual island or World.
    pub(crate) fn try_new_selected_main(
        backend: &crate::skater_manager_local_identity::LocalBackendOwner,
    ) -> Result<Self, SelectedMainWorldError> {
        let flags = if backend
            .try_ghost_enabled()
            .map_err(SelectedMainWorldError::Backend)?
        {
            1
        } else {
            3
        };
        Self::try_new_with_island_flags(860, 360, 540, 2040, flags)
            .map_err(SelectedMainWorldError::Construction)
    }

    pub(crate) fn island_flags(&self) -> u32 {
        self.island_flags
    }

    /// Explicit selected World constructor, retaining the actual `GameInfo` owner
    /// supplied by startup. Never creates a private network flag for a skater.
    pub(crate) fn try_initialize_physical_world_services(
        &mut self,
        game_info: Arc<PhysicalGameInfo>,
        allocated: bool,
    ) -> Result<(), PhysicalWorldServicesError> {
        if self.world_services_attempted {
            return Err(PhysicalWorldServicesError::AlreadyAttempted);
        }
        self.world_services_attempted = true;
        // Simulation::IsNetworkGame reads this same global GameInfo field.
        self.sim_rate
            .lock()
            .map_err(|_| PhysicalWorldServicesError::Poisoned)?
            .bind_game_info(Arc::clone(&game_info));
        self.world_services = Some(PhysicalWorldServices::construct_collection(
            self.contacts.assembly_pool_authority(),
            game_info,
            allocated,
        ));
        Ok(())
    }
    pub(crate) fn physical_world_services(&self) -> Option<&Arc<PhysicalWorldServices>> {
        self.world_services.as_ref()
    }
}

/// Actual mutable `CommonBackEnd::sGameInfo` network field, not an Actor snapshot.
pub(crate) struct PhysicalGameInfo {
    game_mode: Mutex<u32>,
    sync_level: Mutex<i32>,
    network_game: Mutex<bool>,
    ignore_wipeout_reset_button: Mutex<bool>,
    /// `trafficSetting`, `pedestrianSetting` (+0x29C/+0x2A0) and `securitySetting` (+0x2A4),
    /// which `FrontEndState_FreeSkateOption::UpdateGameSettings`8264b268 writes.
    free_skate: Mutex<(f32, f32, bool)>,
    /// The challenge system's invisible boundaries while an online game holds its area, which
    /// `Simulation::TestForNetworkGameCollisionConstraints` 0x8289e510 reads.
    online_boundaries: Mutex<Option<Arc<crate::challenge::invisible_boundary::InvisibleBoundaries>>>,
    /// PC port: the longest a wipeout runs before its auto teleport while an online round of
    /// S.K.A.T.E. or Spot Battle plays; none elsewhere.
    turn_bail_limit: Mutex<Option<f32>>,
    /// The port's online game: the original's sets `isNetworkGame` (`StateOnlineGame::
    /// SetupInitalGameInfo` 0x8275dba4) until `GameModule::BackToSP`, which the port's netcode
    /// leaves clear; the skater's physics asks it as the original's does
    /// ([`PhysicalWorldServices::try_is_network_game`]).
    online_game: Mutex<bool>,
}
impl PhysicalGameInfo {
    /// Selected `GameInfo` ctor82559418 store825594d4, PDB+293.
    pub(crate) fn construct_network_field() -> Arc<Self> {
        Arc::new(Self {
            game_mode: Mutex::new(1),  // GameInfo82559454: GM_SinglePlayer
            sync_level: Mutex::new(0), // GameInfo82559470: SL_None; actual +0x10
            network_game: Mutex::new(false),
            ignore_wipeout_reset_button: Mutex::new(false), //GameInfo825594e8, +298
            free_skate: Mutex::new((1.0, 1.0, true)), // GameInfo825594c8..825594ec
            online_boundaries: Mutex::new(None),
            turn_bail_limit: Mutex::new(None),
            online_game: Mutex::new(false),
        })
    }
    /// The port's online game (see the field).
    pub(crate) fn try_online_game(&self) -> Result<bool, PhysicalWorldServicesError> {
        Ok(*self.online_game.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)?)
    }
    pub(crate) fn try_set_online_game(&self, online: bool) -> Result<(), PhysicalWorldServicesError> {
        *self.online_game.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)? = online;
        Ok(())
    }
    /// The online turn's wipeout limit (see the field).
    pub(crate) fn try_turn_bail_limit(&self) -> Result<Option<f32>, PhysicalWorldServicesError> {
        Ok(*self.turn_bail_limit.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)?)
    }
    pub(crate) fn try_set_turn_bail_limit(&self, limit: Option<f32>) -> Result<(), PhysicalWorldServicesError> {
        *self.turn_bail_limit.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)? = limit;
        Ok(())
    }
    /// The online game's boundaries, while it holds its area.
    pub(crate) fn try_online_boundaries(
        &self,
    ) -> Result<Option<Arc<crate::challenge::invisible_boundary::InvisibleBoundaries>>, PhysicalWorldServicesError> {
        Ok(self.online_boundaries.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)?.clone())
    }
    pub(crate) fn try_set_online_boundaries(
        &self,
        boundaries: Option<Arc<crate::challenge::invisible_boundary::InvisibleBoundaries>>,
    ) -> Result<(), PhysicalWorldServicesError> {
        *self.online_boundaries.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)? = boundaries;
        Ok(())
    }
    /// `UpdateGameSettings`/`StartGameModule`: the game mode the front end starts.
    pub(crate) fn try_set_game_mode(&self, mode: u32) -> Result<(), PhysicalWorldServicesError> {
        *self.game_mode.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)? = mode;
        Ok(())
    }
    /// The free-skate traffic, pedestrian and security settings.
    pub(crate) fn try_free_skate(&self) -> Result<(f32, f32, bool), PhysicalWorldServicesError> {
        Ok(*self.free_skate.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)?)
    }
    pub(crate) fn try_set_free_skate(&self, settings: (f32, f32, bool)) -> Result<(), PhysicalWorldServicesError> {
        *self.free_skate.lock().map_err(|_| PhysicalWorldServicesError::Poisoned)? = settings;
        Ok(())
    }
    /// Actual signed `GameInfo`+0x10, read lazily by `DoCommand825e5610`.
    /// This is the retained field, not a cvar or an offline CRC policy.
    pub(crate) fn try_sync_level(&self) -> Result<i32, PhysicalWorldServicesError> {
        Ok(*self
            .sync_level
            .lock()
            .map_err(|_| PhysicalWorldServicesError::Poisoned)?)
    }
    /// CommonBackEnd.sGameInfo+0, sampled at `OnSimCharacterLoaded82817fc8`.
    pub(crate) fn try_game_mode(&self) -> Result<u32, PhysicalWorldServicesError> {
        Ok(*self
            .game_mode
            .lock()
            .map_err(|_| PhysicalWorldServicesError::Poisoned)?)
    }
    /// `Simulation::IsNetworkGame`8289e448 reads the current global field.
    pub(crate) fn try_is_network_game(&self) -> Result<bool, PhysicalWorldServicesError> {
        Ok(*self
            .network_game
            .lock()
            .map_err(|_| PhysicalWorldServicesError::Poisoned)?)
    }
    /// Same retained GameInfo+298 read at Actor.SetUpPhysics825ff8f0.
    pub(crate) fn try_ignore_wipeout_reset_button(
        &self,
    ) -> Result<bool, PhysicalWorldServicesError> {
        Ok(*self
            .ignore_wipeout_reset_button
            .lock()
            .map_err(|_| PhysicalWorldServicesError::Poisoned)?)
    }
}

/// `World.mPhysicalCollection`+2c. Source entries are non-owning pointers;
/// weak handles retain identity without a World -> Actor -> World Arc cycle.
pub(crate) struct PhysicalWorldServices {
    manager: SimulationManagerContactAuthority,
    game_info: Arc<PhysicalGameInfo>,
    collection: Mutex<Option<Vec<Weak<ActorPhysicalAllocation>>>>,
}
impl PhysicalWorldServices {
    pub(crate) fn game_info(&self) -> &Arc<PhysicalGameInfo> {
        &self.game_info
    }
    pub(crate) fn try_ignore_wipeout_reset_button(
        &self,
    ) -> Result<bool, PhysicalWorldServicesError> {
        self.game_info.try_ignore_wipeout_reset_button()
    }
    /// Selected World ctor827b1754..1798, allocation0x34, `PhysicsSystems`.
    /// Successful fixed vector has exactly nine inline pointer slots and zero
    /// elements; failed allocation publishes null, distinct from an empty list.
    pub(crate) fn construct_collection(
        manager: SimulationManagerContactAuthority,
        game_info: Arc<PhysicalGameInfo>,
        allocated: bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            manager,
            game_info,
            collection: Mutex::new(allocated.then(|| Vec::with_capacity(9))),
        })
    }
    /// `PhysicalPlayerHiLOD::mINetworkGameQuery`'s `Simulation::IsNetworkGame` 0x8289e448, which
    /// the constructor's tail (0x82e04d2c) and `TeleportPlayerNow` (0x82e009c8) ask: an online
    /// game's skater and board collide normally from a spawn or a teleport, without the single
    /// player game's world-only spell. True in the port's online game too.
    pub(crate) fn try_is_network_game(&self) -> Result<bool, PhysicalWorldServicesError> {
        Ok(self.game_info.try_online_game()? || self.game_info.try_is_network_game()?)
    }
    /// `Simulation::Add`828a7c98 appends the same early registered `IPhysical`.
    /// Source does not deduplicate. Capacity overflow's fixed allocator path
    /// is intentionally pending rather than silently growing a native Vec.
    pub(crate) fn try_add(
        &self,
        physical: &Arc<ActorPhysicalAllocation>,
    ) -> Result<(), PhysicalWorldServicesError> {
        if physical.owner().manager() != self.manager {
            return Err(PhysicalWorldServicesError::ForeignWorld);
        }
        let mut collection = self
            .collection
            .lock()
            .map_err(|_| PhysicalWorldServicesError::Poisoned)?;
        let collection = collection
            .as_mut()
            .ok_or(PhysicalWorldServicesError::NullCollection)?;
        if collection.len() == 9 {
            return Err(PhysicalWorldServicesError::PendingFixedCapacityOverflow);
        }
        collection.push(Arc::downgrade(physical));
        Ok(())
    }
    /// `Simulation::Remove` 0x828a63e0, `~PhysicalPlayerHiLOD` 0x82e00f88's last world call:
    /// every entry of the physical leaves the collection (`eastl::remove`, then erase).
    pub(crate) fn try_remove(
        &self,
        physical: &Arc<ActorPhysicalAllocation>,
    ) -> Result<(), PhysicalWorldServicesError> {
        self.collection
            .lock()
            .map_err(|_| PhysicalWorldServicesError::Poisoned)?
            .as_mut()
            .ok_or(PhysicalWorldServicesError::NullCollection)?
            .retain(|entry| !std::ptr::eq(entry.as_ptr(), Arc::as_ptr(physical)));
        Ok(())
    }
    pub(crate) fn try_physical_objects(
        &self,
    ) -> Result<Vec<Arc<ActorPhysicalAllocation>>, PhysicalWorldServicesError> {
        let collection = self
            .collection
            .lock()
            .map_err(|_| PhysicalWorldServicesError::Poisoned)?;
        collection
            .as_ref()
            .ok_or(PhysicalWorldServicesError::NullCollection)?
            .iter()
            .map(|item| {
                item.upgrade()
                    .ok_or(PhysicalWorldServicesError::ExpiredPhysical)
            })
            .collect()
    }
}

#[derive(Debug)]
pub(crate) enum PhysicalWorldServicesError {
    AlreadyAttempted,
    Poisoned,
    ForeignWorld,
    NullCollection,
    ExpiredPhysical,
    PendingFixedCapacityOverflow,
}
impl std::fmt::Display for PhysicalWorldServicesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "physical World services: {self:?}")
    }
}

#[cfg(test)]
mod game_info_tests {
    use super::*;
    #[test]
    fn main_island_captures_backend_ghost_once_before_manager_construction() {
        let backend =
            crate::skater_manager_local_identity::LocalBackendOwner::construct_static_fields(
                PhysicalGameInfo::construct_network_field(),
            );
        let original =
            crate::physical_player_assembly::PhysicalAssemblyWorld::try_new_selected_main(&backend)
                .unwrap();
        assert_eq!(original.island_flags(), 3);
        backend.try_apply_debug_menu_ghost_sample(true).unwrap();
        let later =
            crate::physical_player_assembly::PhysicalAssemblyWorld::try_new_selected_main(&backend)
                .unwrap();
        assert_eq!(later.island_flags(), 1);
        assert_eq!(original.island_flags(), 3);
        backend.try_apply_debug_menu_ghost_sample(false).unwrap();
        assert_eq!(later.island_flags(), 1);
    }

    #[test]
    fn reset_button_field_is_live_on_the_same_static_owner() {
        let owner = PhysicalGameInfo::construct_network_field();
        let same = Arc::clone(&owner);
        assert!(!same.try_ignore_wipeout_reset_button().unwrap());
        *owner.ignore_wipeout_reset_button.lock().unwrap() = true;
        assert!(same.try_ignore_wipeout_reset_button().unwrap());
        assert!(!same.try_is_network_game().unwrap());
    }
}
