//! `FrontEndState_PartyPlay` over the main menu: its character and location lists, the
//! `partyplay` viewer, and the game it hands over.
use super::{
    SelectedLiveError, SelectedLiveSession, SelectedLiveStage, SelectedSceneCharacterVisual,
    career::profile_challenge_system,
    freeskate_viewer::{self, Viewer},
};
use super::super::party_skaters::PartyCharacter;
use crate::{
    challenge::{
        attrib_db::AttribDb,
        signpost_progress,
    },
    edit_skater_profile::EditSkaterProfileStore,
    ui::{
        freeskate_option,
        party_play::{self, Game, PartyGame},
    },
};
use skate2_formats::location::LocationTable;
use std::sync::Arc;

const SIM_LOCATIONS: &str = "data\\content\\world\\stream\\BAM\\BAM_Sim.loc";

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::Visual, e.to_string())
}

/// The PartyPlay screen's viewer and the game its A sends in.
#[derive(Default)]
pub(super) struct PartyPlay {
    viewer: Option<Viewer>,
    /// `EnterGame`, waiting for the fade.
    pub(super) start: Option<PartyGame>,
    /// The game and its location's transform, waiting for the load before the spawns.
    pub(super) spawn: Option<(PartyGame, [f32; 16])>,
    /// The running game, whose players Change Game keeps.
    pub(super) game: Option<PartyGame>,
}

/// The profile's created skater, whose parts record `PlayerLoaderImpl::Load` 0x8281aa80 reads
/// for any `PlayerLoader` member set to it; none without a saved recipe.
fn created_skater_record() -> Result<Option<skate2_formats::skater_profile::CacRecord>, SelectedLiveError> {
    let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
    Ok(store.load_record().map_err(error)?.filter(|record| !record.data.recipe.is_empty()))
}

/// `FormatStringExplicit` with one integer argument: its `{0...}` token reads the number.
fn format_number(format: &str, number: u32) -> String {
    let Some(start) = format.find("{0") else { return format.to_owned() };
    let Some(end) = format[start..].find('}') else { return format.to_owned() };
    format!("{}{number}{}", &format[..start], &format[start + end + 1..])
}

impl<T, C> SelectedLiveSession<T, C> {
    /// `EnterGame` 0x8264ad08's `ResetPlayers` and `LoadPartyMember`s, each of whose
    /// `OnSimCharacterLoaded` 0x82817f28 applies its marquee's character, stance and hat
    /// (`ApplyPlayerAttributes` 0x828158f0; the front end has no Actor to reload), then
    /// `OnUpdate` 0x8264aac8's per-player `SetPlayerCharacter`, `CreatePlayer(i, PT_Human,
    /// peer)` and `SetPlayerControllerIndex`: the peer is -1 for player 0, else the player's
    /// index. A player's created skater takes the profile record's attributes
    /// (`Sk8CACLoader::ApplyAttributesImpl` 0x82819cb8), which an already loaded one does not
    /// apply again.
    pub(super) fn create_party_players(&mut self, game: &PartyGame) -> Result<(), SelectedLiveError> {
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("party play assets absent"))?.mounts);
        let format = super::career::localized(&mounts, "ID_PARTY_PLAYER_PLAYER")?;
        if std::env::var_os("SKATE2_DIAGNOSTIC_PARTY_PLAY").is_some() {
            eprintln!("diagnostic party play name format {format:?}");
        }
        let services = &self.inner.player.as_ref().ok_or_else(|| error("party play player absent"))?
            .selected.owners.services;
        let settings = &services.settings;
        settings.try_reset_players().map_err(error)?;
        for (index, player) in (0_u32..).zip(&game.players) {
            match player.character.marquee {
                Some(marquee) => services.apply_marquee(marquee, index).map_err(error)?,
                None => {
                    if let Some(record) = created_skater_record()? {
                        services.apply_cac_loader_attributes(&record, index).map_err(error)?;
                    }
                }
            }
        }
        for (index, player) in (0_u32..).zip(&game.players) {
            if let Some(marquee) = player.character.marquee {
                settings.try_set_character(index, marquee).map_err(|e| error(format!("{e:?}")))?;
            }
            let peer = if index == 0 { crate::player::NO_PEER_ID } else { i64::from(index) };
            let name = format_number(&format, index + 1);
            settings.try_create_party_human(index, peer, name.as_bytes()).map_err(error)?;
            settings.try_set_controller_index(index, player.controller).map_err(|e| error(format!("{e:?}")))?;
        }
        Ok(())
    }

    /// `UpdatePlayerName` 0x82563a88's `GM_PartyPlay` name for a player:
    /// `FormatString(ID_PARTY_PLAYER_PLAYER, index + 1)`.
    pub(super) fn party_player_name(&self, index: u32) -> Result<String, SelectedLiveError> {
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("party play assets absent"))?.mounts);
        Ok(format_number(&super::career::localized(&mounts, "ID_PARTY_PLAYER_PLAYER")?, index + 1))
    }

    /// `PartyPlayLoadingState` 0x8276e6c8 once the location is loaded: each later player's
    /// `cMsgSpawnPlayer` at `GetSpawnLocation(location, i)` 0x82a8d250 (the location's rows with
    /// its translation moved by (0, 0, -5i): the text location table carries no sub-locations),
    /// then `cMsgSetPlayer(0)` and `SetLocalIndex(0)`.
    pub(super) fn spawn_party_players<E: std::fmt::Debug>(
        &mut self,
        game: &PartyGame,
        location: [f32; 16],
    ) -> Result<(), SelectedLiveError>
    where
        T: FnMut() -> Result<u64, E>,
    {
        let mut moved = false;
        for (index, player) in (0_u32..).zip(&game.players).skip(1) {
            let mut spawn = location;
            spawn[14] += (f64::from(index) as f32) * -5.0;
            let existing = self.inner.party.iter().any(|skater| skater.player_index() == index);
            if existing {
                // A game switched in by Change Game: SpawnPlayer's DestroyPlayer and new Actor
                // become a teleport of the skater the player already has.
                self.apply_retained_skater_spawn_attributes(index)?;
                let queues = &mut self.inner.session_marker.as_mut().ok_or_else(|| error("party play session marker absent"))?.teleport;
                queues.send_set_player(i32::try_from(index).map_err(error)?);
                queues.send_teleport(crate::skeleton_root_transforms::SkeletonRootMatrix::known(spawn), true);
                moved = true;
            } else {
                let record;
                let character = match player.character.marquee {
                    Some(marquee) => PartyCharacter::Marquee(marquee),
                    None => {
                        record = created_skater_record()?.ok_or_else(|| error("party play created skater without a saved recipe"))?;
                        PartyCharacter::Created(&record)
                    }
                };
                self.inner.try_spawn_party_skater(index, character, spawn).map_err(error)?;
            }
            if std::env::var_os("SKATE2_DIAGNOSTIC_PARTY_PLAY").is_some() {
                let character = player.character.marquee.map_or_else(|| "created skater".to_owned(), |marquee| format!("{marquee:#018x}"));
                eprintln!("diagnostic party play {} player {index} ({character}) at {:?}", if existing { "moved" } else { "spawned" }, &spawn[12..15]);
            }
        }
        if moved {
            self.inner.session_marker.as_mut().ok_or_else(|| error("party play session marker absent"))?.teleport.send_set_player(0);
        }
        let owner = self.inner.world.as_ref().ok_or_else(|| error("party play world absent"))?;
        let local = owner
            .world
            .lock()
            .map_err(|_| error("party play world lock poisoned"))?
            .local_skater_identity
            .clone()
            .ok_or_else(|| error("party play local skater identity absent"))?;
        local.try_set_local_index(0).map_err(|e| error(format!("{e:?}")))?;
        self.start_party_play_challenge(game.location.key, game.backend.clone())
    }

    /// Change Game's A on another challenge, once the running one completed: flow message 6's
    /// `StateLoading` (`PartyPlayLoadState` 0x82770ea8 loads the challenge's assets, then
    /// `PartyPlayLoadingState` 0x8276e6c8 spawns every player at its location and starts it).
    /// Like the game's start, the first player travels to the location's start under the
    /// front end's fade (where native shows the tips load screen); the completed challenge's
    /// session goes.
    pub(super) fn switch_party_play(&mut self, location: party_play::Location) -> Result<(), SelectedLiveError> {
        let backend = self.career.as_ref().and_then(|career| career.party_backend()).cloned().ok_or_else(|| error("party play back end absent"))?;
        let mut game = self.party_play.game.clone().ok_or_else(|| error("party play game absent"))?;
        let (start, world) = self.party_challenge_start(location.key)?;
        game.game = backend.game;
        game.location = location;
        game.backend = backend;
        self.drop_career()?;
        self.party_play.game = Some(game.clone());
        self.party_play.spawn = Some((game, start));
        self.freeskate.spawn_attributes_pending = true;
        self.start_world_travel(world, start, true);
        Ok(())
    }

    /// `OnMenuSelectGame` 0x82681338 row 3: the screen `UpdateFEState` pushes once the NIS ends, with
    /// `FrontEndState_PartyPlay::OnEnter` 0x8264a870's lists: the FreeSkate group's skaters and
    /// each game's locations from the profile's challenge system. Unlike the original, whose
    /// `isPartyPlay` list leaves the created skater out, the list keeps it first as FreeSkate's
    /// does, once the profile saved its recipe.
    pub(super) fn open_party_play(&mut self) -> Result<(), SelectedLiveError> {
        let mounts = Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("party play assets absent"))?.mounts);
        if self.freeskate.data.is_none() {
            let db = Arc::new(AttribDb::load_career(&mounts).map_err(error)?);
            let table = LocationTable::parse(&mounts.read_path(SIM_LOCATIONS).map_err(error)?).map_err(error)?;
            self.freeskate.data = Some((db, table));
        }
        let (db, _) = self.freeskate.data.as_ref().expect("front end data loaded");
        let db = Arc::clone(db);
        if self.party_play.viewer.is_none() {
            self.party_play.viewer = Some(Viewer::load(&db, &mounts, freeskate_viewer::PARTY_PLAY).map_err(error)?);
        }
        let system = profile_challenge_system(Arc::clone(&db), &mounts).map_err(error)?;
        let world = system.world();
        let points = |challenge| signpost_progress::challenge_save_data(world, challenge, "ABD Points Earned");
        let name = self
            .inner
            .player
            .as_ref()
            .and_then(|player| player.selected.owners.services.profile.name())
            .unwrap_or_default();
        let created = created_skater_record()?.is_some();
        let characters = freeskate_option::characters(&db, &world.unlockables, created.then_some(name.as_str()));
        let locations = [Game::SpotBattle, Game::Skate, Game::Hom].map(|game| party_play::locations(&db, &world.unlockables, game, &points));
        let menu = self.main_menu.as_mut().ok_or_else(|| error("main menu absent"))?;
        menu.party_play = Some(party_play::Controller::new(characters, locations));
        Ok(())
    }

    /// The PartyPlay screen's intro finished (`FrontEndStateBase::OnIntroDone`).
    pub fn acknowledge_party_play_intro(&mut self) {
        if let Some(party_play) = self.main_menu.as_mut().and_then(|menu| menu.party_play.as_mut()) {
            party_play.intro_done();
        }
    }

    /// The PartyPlay screen's `SetScreenState` calls since the last take.
    pub fn take_party_play_screen_states(&mut self) -> Vec<u32> {
        self.main_menu.as_mut().map(crate::ui::main_menu::Controller::take_party_play_screen_states).unwrap_or_default()
    }

    /// The viewer shows the selected skater while a player picks and hides otherwise; it shuts
    /// down with the screen (`OnLeave` 0x8264a9e0).
    pub(super) fn update_party_play_viewer(&mut self, dt: f32) -> Result<(), SelectedLiveError> {
        let requested = self
            .main_menu
            .as_ref()
            .and_then(|menu| menu.party_play.as_ref())
            .and_then(party_play::Controller::viewer_character);
        let Some(viewer) = self.party_play.viewer.as_mut() else { return Ok(()) };
        let (db, _) = self.freeskate.data.as_ref().ok_or_else(|| error("front end data absent"))?;
        let assets = self.inner.assets.as_ref().ok_or_else(|| error("party play assets absent"))?;
        viewer.update(requested, dt, db, &assets.mounts, &assets.vault, &assets.cvars).map_err(error)
    }

    /// `EnterGame` 0x8264ad08 shuts the viewer down (`SimCharacterViewer::Shutdown`) and hands
    /// the game over once the fade covers.
    pub(super) fn take_party_play_start(&mut self) -> Option<PartyGame> {
        let game = self.party_play.start.take()?;
        if let Some(viewer) = self.party_play.viewer.as_mut() {
            viewer.shutdown();
        }
        Some(game)
    }

    /// The viewer's posed skater, for the front end's scene.
    #[must_use]
    pub fn party_play_viewer_visuals(&self) -> Vec<SelectedSceneCharacterVisual<'_>> {
        self.party_play.viewer.as_ref().map(Viewer::visuals).unwrap_or_default()
    }
}
