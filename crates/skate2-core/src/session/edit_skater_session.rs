//! Host ownership of the native CAC frontend/recipe/profile transaction.
//! Simulation remains paused; preview records never replace saved choices.
use crate::{
    attribute_runtime::AttributeRuntimeVault, edit_skater_character::EditSkaterCharacter,
    edit_skater_frontend::EditSkaterFrontend, edit_skater_profile::EditSkaterProfileStore,
    edit_skater_record, input_cvar_runtime::InputCvarReferences, ui::edit_skater::*,
};
use skate2_formats::{
    cac_parts::CacPartDb, mount::MountSet, recipe::CharacterCustomizations,
    skater_profile::CacRecord,
};
use std::sync::Arc;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
/// What a record wears: its assembled parts and its customizations.
type Worn = ([skate2_formats::skater_profile::CacPartId; 26], CharacterCustomizations);

pub(crate) struct EditSkaterSession {
    pub frontend: EditSkaterFrontend,
    pub character: EditSkaterCharacter,
    pub store: EditSkaterProfileStore,
    db: Arc<CacPartDb>,
    vault: Arc<AttributeRuntimeVault>,
    cvars: InputCvarReferences,
    pub finishing: bool,
    /// `FrontEndState_CaC::mIsOriginallyMale`, `IsCurrentGenderMale` at `InitiateStartup`.
    pub(crate) originally_male: bool,
    hierarchies: [Option<crate::character::CharacterHierarchy>; 2],
    elapsed_millisecond_fraction:f64,
    /// What `character` was built wearing.
    worn: Worn,
}
impl EditSkaterSession {
    /// `FrontEndState_CaC::OnEnter` (`0x82659a08`): the new career's skater starts from
    /// `CACRecord::Init`, any other from the profile's record; the editor reads `cash` and the
    /// clothing unlocks. All gesture choices are available as a PC QoL exception.
    pub fn open(
        mounts: &MountSet,
        vault: Arc<AttributeRuntimeVault>,
        cvars: InputCvarReferences,
        cash: i32,
        unlocked: &impl Fn(u32, &str) -> bool,
        new_unlock: &impl Fn(u32, &str) -> bool,
        new_career: bool,
    ) -> Result<Self> {
        let db = CacPartDb::parse(&mounts.read_path(skate2_formats::cac_parts::DATABASE_PATH)?)?;
        // PC port: a linked Skate 3 data folder's clothing joins Slappy's (`crate::skate3`).
        let skate3 = crate::skate3::linked().and_then(|data| data.clothing_for(&db));
        let skate3_merchandise = skate3.is_some();
        let db = Arc::new(skate3.unwrap_or(db));
        let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path()?)?;
        let record = match store.load_record()? {
            Some(record) if !new_career => record,
            _ => edit_skater_record::fresh_record(&db, &vault, true)?,
        };
        let shop = crate::edit_skater_shop::EditSkaterShop::new(&db, cash, unlocked, new_unlock)?;
        let mut frontend = EditSkaterFrontend::new(
            Arc::clone(&db),
            Arc::clone(&vault),
            record,
            EditSkaterPolicy {
                unlock_all_clothing: false,
            },
            shop,
            new_career,
        )?;
        frontend.controller.skate3_merchandise = skate3_merchandise;
        // PC port: Skate 3's emo hair joined too, so Back Alley Pete's Hair asks which game's.
        frontend.controller.skate3_hair = db.slot_range(2).any(|index| db.is_skate3(index));
        // PC port: the Gesture Set row only with linked Skate 3 data, whose gestures it lists.
        frontend.controller.skate3_gestures = crate::skate3::linked().is_some();
        frontend.set_gesture_settings(store.load_gesture_settings()?)?;
        frontend.set_skate3_style(store.load_skate3_style()?)?;
        let originally_male = frontend.preview_record().data.is_male;
        let worn = Self::worn(&db, &vault, &cvars, frontend.preview_record_mut())?;
        let mut hierarchies = std::array::from_fn(|_| None);
        let hierarchy = Self::cached_hierarchy(
            mounts,
            &mut hierarchies,
            frontend.preview_record().data.is_male,
        )?;
        let character = Self::build_character(mounts, &db, &vault, &cvars, worn.clone(), hierarchy)?;
        Ok(Self {
            frontend,
            character,
            store,
            db,
            vault,
            cvars,
            finishing: false,
            originally_male,
            hierarchies,
            elapsed_millisecond_fraction:0.,
            worn,
        })
    }
    /// What `record` wears: its parts as `assemble_parts` corrects them (binding its colours to
    /// them) and its customizations.
    fn worn(
        db: &CacPartDb,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
        record: &mut CacRecord,
    ) -> Result<Worn> {
        let parts = edit_skater_record::assemble_parts(
            record,
            db,
            vault,
            false,
            cvars.try_cas_correct_rostral().map_err(|e| e.to_string())?,
        )?;
        let parts = crate::edit_skater_character::wearable_parts(db, parts, record.data.is_male);
        let custom = CharacterCustomizations::parse_prefix(record.customizations_bytes()?)?.0;
        Ok((parts, custom))
    }
    fn build_character(
        mounts: &MountSet,
        db: &CacPartDb,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
        (parts, custom): Worn,
        hierarchy: &crate::character::CharacterHierarchy,
    ) -> Result<EditSkaterCharacter> {
        let recipe = crate::edit_skater_character::compose_recipe(db, &parts, custom)?;
        EditSkaterCharacter::load(mounts, recipe, hierarchy, Arc::clone(vault), cvars.clone())
    }
    /// PC port: whether `now` differs from `then` in its colours alone, which the built skater's
    /// materials take without loading it again (`set_colours`).
    fn colours_alone(then: &Worn, now: &Worn) -> bool {
        then.0 == now.0 && CharacterCustomizations { colours: now.1.colours.clone(), ..then.1.clone() } == now.1
    }
    // Recipe binding needs only the authored hierarchy names. Keep one for
    // each sex; the room presentation remains the sole full animation owner.
    fn cached_hierarchy<'a>(
        mounts: &MountSet,
        cache: &'a mut [Option<crate::character::CharacterHierarchy>; 2],
        male: bool,
    ) -> Result<&'a crate::character::CharacterHierarchy> {
        let entry = &mut cache[usize::from(!male)];
        if entry.is_none() {
            let path = if male {
                "data/anim/cac_edit_male.abin"
            } else {
                "data/anim/cac_edit_female.abin"
            };
            let bytes = mounts.read_path(path)?;
            let database = skate2_formats::andale::AndaleDatabase::parse(&bytes)?;
            *entry = Some(crate::character::CharacterHierarchy {
                bone_names: database.hierarchy.bone_names,
            });
        }
        Ok(entry.as_ref().expect("CAC hierarchy populated"))
    }
    pub fn update(
        &mut self,
        mounts: &MountSet,
        input: EditSkaterInput,
        elapsed_seconds: f32,
    ) -> Result<()> {
        if self.finishing {
            return Ok(());
        }
        self.elapsed_millisecond_fraction+=f64::from(elapsed_seconds)*1000.;
        let elapsed_ms=self.elapsed_millisecond_fraction.floor() as u32;
        self.elapsed_millisecond_fraction-=f64::from(elapsed_ms);
        let effects = self.frontend.update(
            input,
            elapsed_ms,
            self.cvars
                .try_cas_preview_delay()
                .map_err(|e| e.to_string())? as u32,
        )?;
        if effects.recipe_changed {
            let worn = Self::worn(&self.db, &self.vault, &self.cvars, self.frontend.preview_record_mut())?;
            // PC port: new colours alone (a colour row's, the colour picker's while its sticks move)
            // go on the built skater's materials. Building it again for each would take its
            // scene's every model and texture from the archives each frame.
            if Self::colours_alone(&self.worn, &worn) {
                self.character.materials.set_colours(worn.1.colours.clone());
            } else {
                let hierarchy = Self::cached_hierarchy(
                    mounts,
                    &mut self.hierarchies,
                    self.frontend.preview_record().data.is_male,
                )?;
                self.character = Self::build_character(mounts, &self.db, &self.vault, &self.cvars, worn.clone(), hierarchy)?;
            }
            self.worn = worn;
        }
        self.finishing = effects.finish;
        Ok(())
    }
    pub fn gameplay_record(&mut self) -> Result<CacRecord> {
        let mut record = self.frontend.record.clone();
        let parts = edit_skater_record::assemble_parts(
            &mut record,
            &self.db,
            &self.vault,
            true,
            self.cvars
                .try_cas_correct_rostral()
                .map_err(|e| e.to_string())?,
        )?;
        let parts = crate::edit_skater_character::wearable_parts(&self.db, parts, record.data.is_male);
        let custom = CharacterCustomizations::parse_prefix(record.customizations_bytes()?)?.0;
        let mut recipe = crate::edit_skater_character::compose_recipe(&self.db, &parts, custom)?;
        recipe.clone_type = 3;
        record.data.recipe = recipe.encode()?;
        Ok(record)
    }
}

#[cfg(test)]
mod board_part_tests {
    use super::*;
    use skate2_formats::{cac_parts::SLOT_NAMES, skater_profile::CacPartId};

    fn original_assets() -> (MountSet, Arc<AttributeRuntimeVault>, InputCvarReferences, CacPartDb) {
        use skate2_formats::mount::{MountedBig, SKATE2_ARCHIVES};
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("game root"));
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES {
            let path = spec.path_under(&root);
            if path.is_file() {
                mounts.push(MountedBig::open(path, spec.use_external_header).unwrap());
            }
        }
        let vault =
            AttributeRuntimeVault::try_load_mounted(&mounts, "data/db/skater.vlt", &crate::selected_original_asset_classes::selected_classes())
                .unwrap();
        let cvars = InputCvarReferences::new_static_initialized();
        crate::input_cvar_runtime::InputCvarManager::new().try_link(&cvars).unwrap();
        let db = CacPartDb::parse(&mounts.read_path(skate2_formats::cac_parts::DATABASE_PATH).unwrap()).unwrap();
        (mounts, vault, cvars, db)
    }

    /// Every part a clothing, accessory or board list offers either sex (`LoadParts`), worn the
    /// way its thumbnail's preview wears it.
    #[test]
    #[ignore = "requires original SKATE2_GAME_ROOT assets"]
    fn every_listed_part_loads_in_the_editor() {
        use EditSkaterOption as O;
        let (mounts, vault, cvars, db) = original_assets();
        let mut hierarchies = std::array::from_fn(|_| None);
        let mut failures = Vec::new();
        let mut tried = 0;
        for male in [true, false] {
            let hierarchy = EditSkaterSession::cached_hierarchy(&mounts, &mut hierarchies, male).unwrap().clone();
            let fresh = edit_skater_record::fresh_record(&db, &vault, male).unwrap();
            for option in [
                O::Hats, O::TShirts, O::ButtonShirts, O::Hoodies, O::Jackets, O::Sweaters, O::Legs, O::Shoes, O::Socks,
                O::Glasses, O::Wrist, O::Necklaces, O::Rings, O::Boards, O::Trucks, O::Wheels,
            ] {
                for index in clothing_parts(&db, option, male, |part| part.attributes[7] == u16::MAX).unwrap() {
                    let part = &db.parts[index];
                    let mut record = fresh.clone();
                    edit_skater_record::wear_part(&mut record, &db, part).unwrap();
                    tried += 1;
                    let built = EditSkaterSession::worn(&db, &vault, &cvars, &mut record)
                        .and_then(|worn| EditSkaterSession::build_character(&mounts, &db, &vault, &cvars, worn, &hierarchy));
                    if let Err(error) = built {
                        failures.push(format!("{} {option:?} {:016x}/{:016x}: {error}", if male { "male" } else { "female" }, part.model_id, part.material_id));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{} of {tried} parts failed: {failures:#?}", failures.len());
    }

    #[test]
    #[ignore = "requires original SKATE2_GAME_ROOT assets"]
    fn every_board_truck_and_wheel_loads_in_the_editor() {
        let (mounts, vault, cvars, db) = original_assets();
        let mut hierarchies = std::array::from_fn(|_| None);
        let mut failures = Vec::new();
        for male in [true, false] {
            let hierarchy = EditSkaterSession::cached_hierarchy(&mounts, &mut hierarchies, male).unwrap().clone();
            for slot in [11_u8, 12, 13] {
                for part in db.parts_in_slot(slot) {
                    let mut record = edit_skater_record::fresh_record(&db, &vault, male).unwrap();
                    record.installed_parts[usize::from(slot)] = CacPartId { model: part.model_id, material: part.material_id };
                    let built = EditSkaterSession::worn(&db, &vault, &cvars, &mut record)
                        .and_then(|worn| EditSkaterSession::build_character(&mounts, &db, &vault, &cvars, worn, &hierarchy));
                    if let Err(error) = built {
                        failures.push(format!("{} {:016x}/{:016x}: {error}", SLOT_NAMES[usize::from(slot)], part.model_id, part.material_id));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }
}
