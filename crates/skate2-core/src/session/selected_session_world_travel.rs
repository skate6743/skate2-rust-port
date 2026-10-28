//! The world a travel lands in: BAM alone, or with an additional world (a downloadable content
//! location's, `fe_locations.WorldFile` not `BAM`) that the travel puts in place before the
//! skater moves there: `GameModule::GameLoad` (0x8273be50) loads the game's additional location
//! (`sGameInfo.additionalWorldLocation`, set by `FrontEndState_FreeSkateOption::UpdateGameSettings`
//! 0x8264b268), `StateLoading::UnloadAdditionalWorldStream` (0x82774428) unloads the one held
//! when the game's location is in BAM or another additional world's, and `LoadAdditionalWorldStream`
//! (0x82772680) loads the next. The port does both under the travel's fade.
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage as Stage};

/// How far [`SelectedLiveSession::try_prepare_world`] has put a world in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorldReadiness {
    Pending,
    Ready,
    /// An additional world the port cannot load: the main world stands in.
    MainInstead,
}

/// What a travel does with the worlds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum TravelWorld {
    /// It keeps the worlds as they are: a teleport within the world held.
    #[default]
    Keep,
    /// BAM alone: a held additional world unloads first.
    Main,
    /// This additional world (`fe_locations.WorldFile`), loaded first, another held one
    /// unloading before it; `location` is the world location (`fe_locations` key) the game then
    /// holds as `sGameInfo.additionalWorldLocation`.
    Additional { file: String, location: u64 },
}

impl TravelWorld {
    /// The world of the world location `location`, whose `WorldFile` is `world`: BAM's is the
    /// main world (`StringNoCaseCompare(world, "BAM")`).
    pub(crate) fn of_location(world: &str, location: u64) -> Self {
        if world.eq_ignore_ascii_case(crate::game::DEBUG_REFERENCE_WORLD_FILE) {
            Self::Main
        } else {
            Self::Additional { file: world.to_owned(), location }
        }
    }
}

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(Stage::TerrainResident, format!("additional world: {e}"))
}

/// `FELocationManager::IsValidLocation`'s test of a world: its Sim stream is mounted, and its name
/// takes no localizer parameter the port lacks (`{0:SponsorWorld}`, a sponsor's training
/// facility's).
fn loadable(mounts: &skate2_formats::mount::MountSet, world: &str) -> bool {
    !world.is_empty()
        && !world.contains('{')
        && mounts.contains_path(&format!("data\\content\\world\\stream\\{world}\\{world}_Sim.xsm"))
}

/// A location of `world` the travel goes to before the world is loaded: `cLocationManager`'s
/// global locators (`LoadGlobalLocations`, the downloadable content's included), then the
/// world's own `<world>_Sim.loc`, which holds the location data its simulation arenas register.
pub(super) fn world_location(mounts: &skate2_formats::mount::MountSet, world: &str, name: &str) -> Result<[f32; 16], SelectedLiveError> {
    let hash = crate::challenge::keys::hash_value32(name);
    if let Some(transform) = super::career::locations::LocationManager::with_global_locations(mounts).map_err(error)?.orientation(hash) {
        return Ok(transform);
    }
    let path = format!("data\\content\\world\\stream\\{world}\\{world}_Sim.loc");
    let bytes = mounts.read_path(&path).map_err(|e| error(format!("{path}: {e}")))?;
    let table = skate2_formats::location::LocationTable::parse(&bytes).map_err(error)?;
    table.find(name).map(|start| start.transform).ok_or_else(|| error(format!("{name} is not a location of {world}")))
}

impl<T, C> SelectedLiveSession<T, C> {
    /// `cTeleportManager::TeleportLocalToWorldLocation` (`0x828a9cf0`): what a teleport into the
    /// world location `location`, whose world is `world_file` (none for the main world), does with
    /// the worlds; none when it needs no world operation (`RequiresAdditionalWorldOperation`), an
    /// ordinary teleport.
    pub(super) fn world_operation(&self, location: u64, world_file: Option<&str>) -> Option<TravelWorld> {
        let target = TravelWorld::of_location(world_file.unwrap_or(crate::game::DEBUG_REFERENCE_WORLD_FILE), location);
        let terrain = self.inner.terrain.as_ref()?;
        let held = terrain.additional_world().filter(|_| !terrain.additional_world_leaving());
        let in_place = match (&target, held) {
            (TravelWorld::Main, None) => true,
            (TravelWorld::Additional { file, .. }, Some(held)) => file.eq_ignore_ascii_case(held),
            _ => false,
        };
        (!in_place).then_some(target)
    }

    /// Whether a travel's additional world is one the port can load (see [`loadable`]).
    pub(super) fn additional_world_loadable(&self, world: &TravelWorld) -> bool {
        match world {
            TravelWorld::Additional { file, .. } => self.inner.assets.as_ref().is_some_and(|assets| loadable(&assets.mounts, file)),
            TravelWorld::Keep | TravelWorld::Main => true,
        }
    }

    /// The travel behind the load screen (`StateMessage_TeleportToPosition`) to `transform`, the
    /// worlds put in place first.
    pub(super) fn start_world_travel(&mut self, world: TravelWorld, transform: [f32; 16], on_board: bool) {
        self.frontend_fade.cover();
        self.map_travel_world = world;
        self.map_travel = Some(transform);
        self.map_travel_presentation_ready = false;
        self.map_travel_dispatched = false;
        self.map_travel_on_board = on_board;
    }

    /// One step of putting the travel's world in place for a travel to `destination`; true once
    /// it is. A world the port cannot load leaves the travel in the main world.
    pub(super) fn try_prepare_travel_world(&mut self, destination: [f32; 3]) -> Result<bool, SelectedLiveError> {
        let world = self.map_travel_world.clone();
        Ok(match self.try_prepare_world(&world, destination)? {
            WorldReadiness::Pending => false,
            WorldReadiness::Ready => true,
            WorldReadiness::MainInstead => {
                self.map_travel_world = TravelWorld::Main;
                true
            }
        })
    }

    /// One step of putting `world` in place around `destination` (a travel's, or a saved replay's
    /// as `StateReplay::LoadAdditionalWorld` 0x827756e0 loads it). A held world it does not want
    /// leaves first: the simulation focus moves to the destination, so its collections depart,
    /// and the presentation releases its own.
    pub(super) fn try_prepare_world(&mut self, world: &TravelWorld, destination: [f32; 3]) -> Result<WorldReadiness, SelectedLiveError> {
        let presentation_holds = self.presentation_additional_world.clone();
        let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("terrain absent"))?;
        let target = match world {
            TravelWorld::Keep => return Ok(WorldReadiness::Ready),
            TravelWorld::Main => None,
            TravelWorld::Additional { file, location } => Some((file.clone(), *location)),
        };
        if let Some(held) = terrain.additional_world().map(str::to_owned) {
            if target.as_ref().is_some_and(|(file, _)| *file == held) {
                terrain.cancel_additional_world_unload();
            } else {
                terrain.request_additional_world_unload();
            }
            if terrain.additional_world_leaving() {
                terrain.try_update_focus(destination).map_err(error)?;
                let holds = presentation_holds.as_deref() == Some(held.as_str());
                if !terrain.try_update_additional_world_unload(holds).map_err(error)? {
                    return Ok(WorldReadiness::Pending);
                }
                eprintln!("travel: additional world {held} unloaded");
                self.additional_location = None;
            }
        }
        let Some((file, location)) = target else { return Ok(WorldReadiness::Ready) };
        let mounts = &self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts;
        if !loadable(mounts, &file) {
            // A world the port cannot load: the main world stands in, which lands a travel where
            // the main world holds the destination, else where the skater is.
            eprintln!("travel: additional world {file} cannot load; the main world stands in");
            return Ok(WorldReadiness::MainInstead);
        }
        let terrain = self.inner.terrain.as_mut().ok_or_else(|| error("terrain absent"))?;
        if terrain.additional_world().is_none() {
            terrain.try_start_additional_world(&file).map_err(error)?;
            eprintln!("travel: additional world {file} loading");
        }
        let loaded = terrain.try_poll_additional_world().map_err(error)?;
        if loaded {
            self.additional_location = Some(location);
        }
        Ok(if loaded { WorldReadiness::Ready } else { WorldReadiness::Pending })
    }

    /// Whether the travel's world is in place and `presented` (the additional world the
    /// presentation that reports ready selected from) is the one loaded.
    pub(super) fn travel_world_presented(&self, presented: Option<&str>) -> bool {
        let Some(terrain) = self.inner.terrain.as_ref() else { return false };
        let loaded = self.loaded_additional_world();
        let in_place = match &self.map_travel_world {
            TravelWorld::Keep => true,
            TravelWorld::Main => terrain.additional_world().is_none(),
            TravelWorld::Additional { file, .. } => loaded == Some(file.as_str()),
        };
        in_place && presented == loaded
    }

    /// The front end's world is BAM: an additional world a game left behind unloads.
    pub(super) fn try_unload_frontend_additional_world(&mut self) -> Result<(), SelectedLiveError> {
        let presentation_holds = self.presentation_additional_world.clone();
        let Some(terrain) = self.inner.terrain.as_mut() else { return Ok(()) };
        let Some(held) = terrain.additional_world().map(str::to_owned) else { return Ok(()) };
        terrain.request_additional_world_unload();
        if terrain.try_update_additional_world_unload(presentation_holds.as_deref() == Some(held.as_str())).map_err(error)? {
            eprintln!("front end: additional world {held} unloaded");
            self.additional_location = None;
        }
        Ok(())
    }

    /// The additional world loaded with BAM, whose Pres and Tex files the presentation's focus
    /// selects from as well.
    #[must_use]
    pub fn loaded_additional_world(&self) -> Option<&str> {
        let terrain = self.inner.terrain.as_ref()?;
        terrain.additional_world().filter(|_| terrain.additional_world_loaded())
    }

    /// `cMsgSetWorldRenderLocation`'s key: `sGameInfo.additionalWorldLocation`, which
    /// `LoadAdditionalWorldStream` (0x82772680), `TeleportLoadState` (0x8275fdb8) and `GameLoad`
    /// (0x8273be50) send and `LooseWorldPresentation::SetWorldRenderLocation` (0x827d53a8) looks
    /// up as a `render_locations` collection; none in the main world.
    #[must_use]
    pub fn world_render_location(&self) -> Option<u64> {
        self.additional_location
    }

    /// The additional world the presentation's streams hold collections of, which a held world's
    /// unload waits to leave.
    pub fn acknowledge_presentation_additional_world(&mut self, held: Option<String>) {
        self.presentation_additional_world = held;
    }
}
