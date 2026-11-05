//! An online game's dynamic objects as the original's network game has them. Its living world
//! spawns nothing: `Simulation::IsLWSpawnEnabled` (0x8289e780) is false with `isNetworkGame` and
//! `sGhostSystemEnabled` set (`StateOnlineGame::SetupInitalGameInfo` 0x8275db08), which
//! `LivingWorld::Manager::Reset` (0x8271d3d8) keeps as the census's allow bit. Its objects are
//! its challenge hull's: `GameModule::GameLoad` (0x8273be50) loads the challenge's assets with
//! asset flags 7 (0x8273cc44), `RegDMOArena` (0x828a0420) hands their DMO data to
//! `Dmo::Registration`, and each `Simulation::Reset` (0x8289eec8) has `Dmo::SimManager` place
//! the records inside the game's boundary as static bodies (`SpawnStatic` 0x825eb0a0): solid and
//! grindable, and nothing moves or grabs them. The port's online game, whose netcode is its own,
//! leaves the native network flags clear, so this does their part.

use super::{SelectedLiveError, SelectedLiveSession, error, trace};
use crate::challenge::assets::{challenge_hull_names, hull_id};
use crate::living_world_grind_services::dmo_locators::LocatorServices as _;

/// `GameModule::GameLoad`'s asset flags for the network game's challenge.
const ONLINE_ASSET_FLAGS: u32 = 7;

/// The online game's static objects and the hulls they come from.
#[derive(Default)]
pub(super) struct OnlineDmos {
    /// The network game's living world reset ran for the game in play.
    reset: bool,
    /// The challenge whose area (its boundary) the game holds, and its name.
    area: Option<(u64, String)>,
    /// The challenge whose hulls are held, and their IDs and names.
    challenge: Option<u64>,
    hulls: Vec<(u32, String)>,
    /// The static objects out, and the locator groups they came from.
    spawned: Vec<u32>,
    groups: Vec<u64>,
}

impl OnlineDmos {
    /// A game entered: its first tick resets the living world as a network game.
    pub(super) fn begin_game(&mut self) {
        self.reset = false;
        self.spawned.clear();
        self.groups.clear();
    }

    /// The challenge whose area the game now holds.
    pub(super) fn hold_area(&mut self, challenge: u64, name: String) {
        self.area = Some((challenge, name));
    }
}

impl<T, C> SelectedLiveSession<T, C> {
    /// A completed tick in an online game: the network game's living world reset once, its
    /// challenge's hulls held, and `SpawnStatic`'s objects out.
    pub(in super::super) fn update_online_dmos(&mut self) -> Result<(), SelectedLiveError> {
        if !self.online.game.as_ref().is_some_and(|game| game.entered) {
            return Ok(());
        }
        if !self.online.dmos.reset {
            self.online.dmos.reset = true;
            self.online.dmos.spawned.clear();
            self.online.dmos.groups.clear();
            self.inner
                .try_with_selected_queued_frame(|frame, _, _| {
                    let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                        return Ok(());
                    };
                    living
                        .try_reset_game(frame.world, frame.actor, frame.cvars, recipes, true)
                        .map_err(|e| format!("online LivingWorld reset: {e}"))
                })
                .map_err(|e| error(format!("{e:?}")))?
                .map_err(error)?;
        }
        self.hold_online_dmo_hulls()?;
        self.place_online_dmos()
    }

    /// Whether the online game holds the hull through its rounds: a round's challenge system
    /// unloading its assets leaves it.
    pub(in super::super) fn online_holds_hull(&self, hull: u32) -> bool {
        self.online.dmos.hulls.iter().any(|&(held, _)| held == hull)
    }

    /// `BackToSP` (0x8273da50): the game's hulls go. Its static objects go at the next
    /// `Simulation::Reset` (off the ghost system `Dmo::SimManager::Reset` 0x825eb5a8 runs
    /// `UnspawnAll`), the living world reset of the port's game unload.
    pub(super) fn release_online_dmos(&mut self) -> Result<(), SelectedLiveError> {
        let hulls = std::mem::take(&mut self.online.dmos).hulls;
        if let Some(terrain) = self.inner.terrain.as_mut() {
            for (hull, _) in hulls {
                terrain.unload_hull(hull).map_err(error)?;
            }
        }
        Ok(())
    }

    /// `LoadChallengeAssets(challengeKey, true, false, 7)` for the challenge whose area the game
    /// holds: its hulls load with asset flags 7 and a former area's other hulls go. A hull the
    /// loader no longer holds (an earlier game's unload took it) is asked for again.
    fn hold_online_dmo_hulls(&mut self) -> Result<(), SelectedLiveError> {
        let area = self.online.dmos.area.clone();
        if self.online.dmos.challenge != area.as_ref().map(|&(challenge, _)| challenge) {
            let wanted: Vec<(u32, String)> = match &area {
                // PC port: a linked Skate 3 data's game holds none of its own hulls' static
                // objects, whose recipes are Skate 3's.
                Some((challenge, _)) if crate::skate3::linked().is_some_and(|data| data.online_spot(*challenge).is_some()) => Vec::new(),
                Some((challenge, name)) => {
                    let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("online data absent"))?;
                    challenge_hull_names(db, *challenge, name).into_iter().map(|name| (hull_id(&name), name)).collect()
                }
                None => Vec::new(),
            };
            let held = std::mem::take(&mut self.online.dmos.hulls);
            let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("online terrain absent"))?;
            for (hull, _) in held {
                if !wanted.iter().any(|&(id, _)| id == hull) {
                    terrain.unload_hull(hull).map_err(error)?;
                }
            }
            trace(format_args!("online DMOs: hulls {:?}", wanted.iter().map(|(_, name)| name.as_str()).collect::<Vec<_>>()));
            self.online.dmos.hulls = wanted;
            self.online.dmos.challenge = area.map(|(challenge, _)| challenge);
        }
        let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("online terrain absent"))?;
        for (hull, name) in &self.online.dmos.hulls {
            if !terrain.holds_hull(*hull) {
                terrain.load_hull(name, false, ONLINE_ASSET_FLAGS).map_err(error)?;
            }
        }
        Ok(())
    }

    /// `SpawnStatic` again once the groups `RegDmoLocator` holds change or one of its objects is
    /// gone (a living world reset culled it): the old objects go, then the groups' records inside
    /// the game's boundary come out.
    fn place_online_dmos(&mut self) -> Result<(), SelectedLiveError> {
        let Some(boundaries) = self.online.boundaries.clone() else { return Ok(()) };
        // PC port: a linked Skate 3 data's game spawns no static objects; its district's groups
        // name Skate 3's recipes.
        if self.online_game_is_skate3() {
            return Ok(());
        }
        let owner = self.inner.world.as_ref().ok_or_else(|| error("online world absent"))?;
        let terrain = self.inner.terrain.as_ref().ok_or_else(|| error("online terrain absent"))?;
        let startup = terrain.terrain_for(&owner.world).map_err(|e| error(format!("{e:?}")))?;
        let groups = startup.static_dmo_groups().map_err(error)?;
        let present = {
            let world = owner.world.lock().map_err(|_| error("world lock poisoned"))?;
            crate::selected_living_world_dmo::SelectedLivingWorldDmo::online_dmos_present(&world, &self.online.dmos.spawned)
                .map_err(error)?
        };
        if groups == self.online.dmos.groups && present {
            return Ok(());
        }
        let Some(chunks) = startup.static_dmo_chunks().map_err(error)? else { return Ok(()) };
        let mut services = chunks.services();
        let mut records = Vec::new();
        for &group in &chunks.groups {
            if let Some(dmos) = services.locator_chunk(group).map_err(error)? {
                records.extend(dmos);
            }
        }
        let previous = std::mem::take(&mut self.online.dmos.spawned);
        let spawned = self
            .inner
            .try_with_selected_queued_frame(|frame, _, _| {
                let (Some(living), Some(recipes)) = (frame.living_world, frame.living_world_recipes.as_ref()) else {
                    return Ok(Vec::new());
                };
                living
                    .try_unspawn_online_dmos(frame.world, frame.actor, frame.cvars, recipes, &previous)
                    .map_err(|e| format!("online DMOs going: {e}"))?;
                living
                    .try_spawn_online_dmos(frame.world, frame.actor, frame.cvars, recipes, &records, &boundaries)
                    .map_err(|e| format!("online DMOs: {e}"))
            })
            .map_err(|e| error(format!("{e:?}")))?
            .map_err(error)?;
        trace(format_args!("online DMOs: {} static from {} locator groups", spawned.len(), groups.len()));
        self.online.dmos.spawned = spawned;
        self.online.dmos.groups = groups;
        Ok(())
    }
}
