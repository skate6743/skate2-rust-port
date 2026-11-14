//! Main-menu Extras, native actions and profile save (8265a700 / 82643168).
use super::{SelectedLiveError, SelectedLiveSession, SelectedLiveStage, career};
use crate::{
    challenge::unlockables::{self, Unlockables},
    edit_skater_profile::{AudioProfile, EditSkaterProfileStore},
    ui::extras::{self, Action, Cheat, Page, TextInput},
};
use skate2_formats::{cac_parts::CacPartDb, skater_profile::ProfileData};

fn error(e: impl std::fmt::Display) -> SelectedLiveError {
    SelectedLiveError::failed(SelectedLiveStage::PreupdateInput, format!("Extras: {e}"))
}
impl<T, C> SelectedLiveSession<T, C> {
    pub(super) fn open_extras(&mut self) -> Result<(), SelectedLiveError> {
        let db = self.movie_attrib_db()?;
        let store =
            EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?)
                .map_err(error)?;
        let mut unlocks = Unlockables::new(&db);
        unlocks.load(career::leaf_tuples(
            &store
                .load_leaf(unlockables::SAVABLE, unlockables::DATA)
                .map_err(error)?,
        ));
        let film = unlocks.is_unlocked(
            unlockables::category::REPLAY,
            unlockables::string_hash("filmpackmovie"),
            true,
        );
        if let Some(menu) = self.main_menu.as_mut() {
            menu.extras = Some(extras::Controller::new(film));
        }
        Ok(())
    }
    pub(super) fn update_extras_action(&mut self, action: Action) -> Result<(), SelectedLiveError> {
        match action {
            Action::None => {}
            Action::Back => {
                if let Some(menu) = self.main_menu.as_mut() {
                    // PopState re-enters MainMenu, whose picker starts at zero.
                    menu.extras = None;
                    menu.selected = 0;
                    menu.intro_complete = false;
                }
            }
            Action::Movie(key) => self.start_extras_movie(key, false),
            Action::Credits => self.start_extras_movie(extras::CREDITS_MOVIE, true),
            Action::Cheat(cheat) => {
                let saved = self.execute_extras_cheat(cheat);
                if let Some(extras) = self.main_menu.as_mut().and_then(|m| m.extras.as_mut()) {
                    extras.state.page = match saved {
                        Ok(()) => Page::Result(cheat),
                        Err(e) => Page::SaveError(e.to_string()),
                    };
                }
            }
            Action::MaxOut => {
                let saved = self.max_out_save();
                if let Some(extras) = self.main_menu.as_mut().and_then(|m| m.extras.as_mut()) {
                    extras.state.page = match saved {
                        Ok(()) => Page::MaxedOut,
                        Err(e) => Page::SaveError(e.to_string()),
                    };
                }
            }
        }
        Ok(())
    }
    /// PC port addition: Extras' Max Out Save. The profile's career, read as the career's start
    /// reads it, is maxed out (`challenge::max_out`) and the created skater's inventory takes every
    /// piece the shop sells; the career's savables and the record are written to SKATER.P in one
    /// save. The profile's other savables, its new-profile flag and its story choices stay as
    /// they are.
    fn max_out_save(&mut self) -> Result<(), SelectedLiveError> {
        use crate::challenge::{dynamic_hull, max_out, signpost_progress};
        let db = self.movie_attrib_db()?;
        let mounts = std::sync::Arc::clone(&self.inner.assets.as_ref().ok_or_else(|| error("assets absent"))?.mounts);
        let mut system = career::profile_challenge_system(db, &mounts).map_err(error)?;
        let path = EditSkaterProfileStore::default_path().map_err(error)?;
        let mut store = EditSkaterProfileStore::open(path).map_err(error)?;
        // Sk8ChallengeSystem::LoadData 0x825a2ed8, as the career's start reads it.
        let hulls = store.load_leaf(dynamic_hull::SAVABLE, dynamic_hull::DATA).map_err(error)?;
        system.world_mut().dynamic_hulls.load(career::leaf_tuples(&hulls));
        // The catalog Edit Skater's shop sells from, with a linked Skate 3 data folder's clothing.
        let catalog = CacPartDb::parse(&mounts.read_path(skate2_formats::cac_parts::DATABASE_PATH).map_err(error)?).map_err(error)?;
        let skate3 = crate::skate3::linked().and_then(|data| data.clothing_for(&catalog));
        let catalog = skate3.unwrap_or(catalog);
        let (unlocks, sponsors) = crate::edit_skater_shop::clothing_unlocks(&catalog).map_err(error)?;
        max_out::max_out(system.world_mut(), &max_out::Clothing { unlocks, sponsors });
        let mut record = store.load_record().map_err(error)?;
        let (owned, missed) = match record.as_mut() {
            Some(record) => crate::edit_skater_shop::own_everything(&catalog, record).map_err(error)?,
            None => (0, 0),
        };
        let world = system.world();
        let leaves = signpost_progress::save(world);
        let unlock_data = world.unlockables.save();
        let hull_data = world.dynamic_hulls.save();
        let data = match store.load_profile_data().map_err(error)? {
            Some(data) => data,
            None => ProfileData::reset(
                &career::localized(&mounts, "ID_DEFAULT_PLAYER_NAME")?,
                &career::localized(&mounts, "ID_DEFAULT_PLAYER_NICKNAME")?,
            ),
        };
        let audio = AudioProfile {
            settings: &self.audio.settings,
            music_player: self.audio.manager.as_ref().map(|m| (&m.profile, m.catalogue())),
        };
        store
            .save_memcard_and_record(
                &data,
                Some(&audio),
                &[
                    (unlockables::SAVABLE, &[], &unlock_data),
                    (dynamic_hull::SAVABLE, &dynamic_hull::SAVABLES_BEFORE, &hull_data),
                    (signpost_progress::SAVABLE, &signpost_progress::SAVABLES_BEFORE, &leaves),
                ],
                record.as_ref(),
            )
            .map_err(error)?;
        eprintln!(
            "Extras: savegame maxed out (cash {}, {} challenges completed, {} of {} unlock slots, {owned} parts joined the inventory, {missed} did not fit)",
            signpost_progress::cash(world),
            world.signpost.completed.len(),
            world.unlockables.items().len(),
            unlockables::CAPACITY,
        );
        // The return to the main menu dropped the last game's career; one still held would save
        // its older career over this one.
        if self.career.is_some() {
            self.drop_career()?;
        }
        Ok(())
    }
    fn execute_extras_cheat(&mut self, cheat: Cheat) -> Result<(), SelectedLiveError> {
        let db = self.movie_attrib_db()?;
        let path = EditSkaterProfileStore::default_path().map_err(error)?;
        let mut store = EditSkaterProfileStore::open(path).map_err(error)?;
        let mut unlocks = Unlockables::new(&db);
        unlocks.load(career::leaf_tuples(
            &store
                .load_leaf(unlockables::SAVABLE, unlockables::DATA)
                .map_err(error)?,
        ));
        let mut reset_tick = None;
        match cheat {
            Cheat::BigBlack => {
                unlocks.unlock(
                    unlockables::category::CHARACTERS,
                    unlockables::string_hash("big_black"),
                    false,
                );
            }
            Cheat::Stereoscopy => self.stereoscopy = !self.stereoscopy,
            Cheat::StreetSweeper => reset_tick = Some(self.reset_extras_dynamic_objects()?),
        }
        let mounts = &self
            .inner
            .assets
            .as_ref()
            .ok_or_else(|| error("assets absent"))?
            .mounts;
        let data = match store.load_profile_data().map_err(error)? {
            Some(data) => data,
            None => ProfileData::reset(
                &career::localized(mounts, "ID_DEFAULT_PLAYER_NAME")?,
                &career::localized(mounts, "ID_DEFAULT_PLAYER_NICKNAME")?,
            ),
        };
        let audio = AudioProfile {
            settings: &self.audio.settings,
            music_player: self
                .audio
                .manager
                .as_ref()
                .map(|m| (&m.profile, m.catalogue())),
        };
        // Do not reset new_profile or career state: a cheat is valid before a career exists.
        // The store merges unknown savables, validates the codec and atomically commits SKATER.P.
        let unlock_data = unlocks.save();
        let reset_data = [(
            "DynamicObjectsBackEnd::Data",
            3,
            vec![
                0i32.to_be_bytes().to_vec(),
                reset_tick.unwrap_or(0).to_be_bytes().to_vec(),
            ],
        )];
        let mut savables = vec![(unlockables::SAVABLE, &[][..], &unlock_data[..])];
        if reset_tick.is_some() {
            savables.push(("DYNAMICOBJECTS", &[], &reset_data));
        }
        store
            .save_memcard(&data, Some(&audio), &savables)
            .map_err(error)?;
        // A retained backend must not overwrite the cheat with its older unlock set.
        if cheat == Cheat::BigBlack {
            if let Some(career) = self.career.as_mut() {
                career.system.world_mut().unlockables.unlock(
                    unlockables::category::CHARACTERS,
                    unlockables::string_hash("big_black"),
                    false,
                );
            }
        }
        Ok(())
    }
    fn reset_extras_dynamic_objects(&mut self) -> Result<u32, SelectedLiveError> {
        use crate::selected_living_world_recipes::{
            SelectedRecipeFrame, pedestrian_animation::PedestrianSessionAnimation,
        };
        use std::sync::Arc;
        let world_owner = self
            .inner
            .world
            .as_ref()
            .ok_or_else(|| error("world absent"))?;
        let terrain = self
            .inner
            .terrain
            .as_ref()
            .ok_or_else(|| error("terrain absent"))?;
        let startup = terrain
            .terrain_for(&world_owner.world)
            .map_err(|e| error(format!("{e:?}")))?;
        let chunks = startup
            .challenge_dmo_chunks()
            .map_err(error)?
            .ok_or_else(|| error("dynamic object chunks absent"))?;
        let owners = &mut self
            .inner
            .player
            .as_mut()
            .ok_or_else(|| error("player absent"))?
            .selected
            .owners;
        let cvars = owners.cvars.clone();
        let recipes = Arc::clone(&owners.living_world_recipes);
        let mut runtime = recipes.lock().map_err(|_| error("recipe lock"))?;
        let graphs = Arc::clone(
            owners
                .graphs
                .as_ref()
                .ok_or_else(|| error("graphs absent"))?,
        );
        let graphs = graphs.lock().map_err(|_| error("graph lock"))?;
        let frame = SelectedRecipeFrame {
            challenge: None,
            triggers: &startup.triggers,
            pedestrian_animation: PedestrianSessionAnimation {
                startup: &owners.pedestrian_animation,
                shared: &owners.animation,
                world: &world_owner.world,
            },
            runtime: &mut runtime,
            registry: owners
                .recipe_registry
                .as_mut()
                .ok_or_else(|| error("recipe registry absent"))?,
            requests: owners
                .recipe_load
                .as_mut()
                .ok_or_else(|| error("recipe loader absent"))?
                .requests_mut(),
            mounts: &owners.mounts,
            vault: &owners.services.vault,
            graphs: &graphs,
            painter: &startup.layers,
        };
        let mut world = world_owner.world.lock().map_err(|_| error("world lock"))?;
        let living = Arc::clone(world.grind_services.living_world());
        let mut living = living.lock().map_err(|_| error("LivingWorld lock"))?;
        let painter = startup
            .layers
            .lock()
            .map_err(|_| error("WorldPainter lock"))?;
        frame
            .with_dmo_culls(
                &mut living,
                &mut world,
                &cvars,
                &painter,
                |objects, locators, cull| {
                    for id in objects.ids() {
                        cull(objects, locators, id)?;
                    }
                    locators
                        .reset_all(&mut chunks.services())
                        .map_err(|e| format!("reset locators: {e:?}"))?;
                    Ok(locators.tick())
                },
            )
            .map_err(error)
    }
    /// Mandatory PC first-use prompt; original ProfileData v3 persistence.
    pub(super) fn save_first_profile_name(&mut self, name: &str) {
        let result=(|| -> Result<(),SelectedLiveError> {
            let mut store=EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(error)?).map_err(error)?;
            let mounts=&self.inner.assets.as_ref().ok_or_else(||error("name prompt assets absent"))?.mounts;
            let mut data=match store.load_profile_data().map_err(error)? {
                Some(data)=>data,
                None=>ProfileData::reset(
                    &career::localized(mounts,"ID_DEFAULT_PLAYER_NAME")?,
                    &career::localized(mounts,"ID_DEFAULT_PLAYER_NICKNAME")?,
                ),
            };
            // SetProfileName825647c0: StringnCopy(...,31); nickname and
            // mNewProfile retain ResetProfile's values until Career starts.
            data.name=[0;32];
            for (dst,src) in data.name[..31].iter_mut().zip(name.bytes()) {*dst=src;}
            let audio=AudioProfile {settings:&self.audio.settings,
                music_player:self.audio.manager.as_ref().map(|m|(&m.profile,m.catalogue()))};
            store.save_memcard(&data,Some(&audio),&[]).map_err(error)?;
            let services=&self.inner.player.as_ref().ok_or_else(||error("name prompt player absent"))?.selected.owners.services;
            services.profile.set_saved_name(data.name).map_err(error)?;
            services.settings.try_update_profile_name(&services.profile).map_err(error)?;
            Ok(())
        })();
        if let Some(menu)=self.main_menu.as_mut() {
            match result {
                Ok(())=>menu.name_prompt=None,
                Err(e)=>if let Some(prompt)=menu.name_prompt.as_mut() {
                    prompt.error=Some(format!("Could not save your name: {e}"));
                },
            }
        }
    }
    pub fn extras_text_entry_active(&self) -> bool {
        if self.online_chat_open() || self.online_nickname_prompt_open() {
            return true;
        }
        if self.main_menu.as_ref().is_some_and(|m|m.name_prompt.is_some() && m.settled && m.intro_complete) {
            return true;
        }
        if self.main_menu.as_ref().is_some_and(crate::ui::main_menu::Controller::nickname_prompt_open) {
            return true;
        }
        self.main_menu
            .as_ref()
            .and_then(|m| m.extras.as_ref())
            .is_some_and(|e| {
                matches!(
                    e.state.page,
                    Page::Code { .. } | Page::Result(_) | Page::SaveError(_)
                )
            })
    }
    pub fn extras_text_input(&mut self, input: TextInput) {
        if self.online_chat_open() {
            self.online_chat_input(input);
            return;
        }
        if self.online_nickname_prompt_open() {
            self.online_nickname_input(input);
            return;
        }
        if let Some(menu)=self.main_menu.as_mut()
            && menu.settled && menu.intro_complete
            && let Some(prompt)=menu.name_prompt.as_mut() {
            prompt.input(input);
            return;
        }
        if let Some(menu)=self.main_menu.as_mut()
            && menu.nickname_prompt_open()
            && let Some(prompt)=menu.nickname_prompt.as_mut() {
            prompt.input(input);
            return;
        }
        if let Some(extras) = self.main_menu.as_mut().and_then(|m| m.extras.as_mut()) {
            extras.text_input(input);
        }
    }
    pub fn acknowledge_extras(&mut self, intro: bool, credits_intro: bool, credits_done: bool) {
        if let Some(extras) = self.main_menu.as_mut().and_then(|m| m.extras.as_mut()) {
            extras.intro_complete |= intro;
        }
        self.acknowledge_credits(credits_intro, credits_done);
    }
    /// Stereoscopy is a runtime render toggle in the reference, not an unlock or saved option.
    pub fn stereoscopy_enabled(&self) -> bool {
        self.stereoscopy
    }
}
