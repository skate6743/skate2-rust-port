//! `Character::SimCharacterViewer` (`Startup` 0x828174d8, `ShowCharacter` 0x82817540, `Shutdown`
//! 0x828175e8) and the presentation's `Character::MVManager` for the FreeSkate screen: the
//! selected skater on the `freeskate` `fe_character_location`, looping that location's clip from
//! `data\anim\marquee_viewer.abin`.
use super::{
    SelectedSceneCharacterVisual,
    scene_characters::{SceneCharacterModels, recipe_model},
};
use crate::{
    attribute_runtime::AttributeRuntimeVault,
    challenge::attrib_db::AttribDb,
    edit_skater_profile::EditSkaterProfileStore,
    input_cvar_runtime::InputCvarReferences,
    nis::characters::{SKATER_FACE, SceneAnimData, SceneCharacter, SceneCharacterAnim},
    ui::freeskate_option::CHARACTERS_MARQUEE,
};
use skate2_formats::{andale::AndaleDatabase, attrib::string_hash64, mount::MountSet, recipe::CharacterRecipe};
use std::sync::Arc;

/// `ViewerAnimationLibrary::Load` 0x8259ac08: `data\anim\<db>.abin` for `ViewerStartup`
/// 0x8281fa18's "marquee_viewer".
const ANIM_DB: &str = "data\\anim\\marquee_viewer.abin";
/// The `fe_character_location`s the screens start the viewer on: FreeSkateOption's
/// `UpdateCharacter` 0x8265b4d0 and PartyPlay's `OnIntroDone` 0x8264b068.
pub(super) const FREESKATE: u64 = 0xF17B_6509_256E_E58B;
pub(super) const PARTY_PLAY: u64 = 0x4042_4A7D_0C50_AEF8;
/// `ViewerStartup`'s animation name attribute.
const ANIMATION: u64 = 0x4B82_FD78_FFEC_71D0;
/// `MarqueeLoader` 0x82d0b180: `CreateMarqueeLoader`'s `NOT_LOCAL_PLAYER` takes `CloneType_NoClone`;
/// the local player's loader, which the created skater shares, `CloneType_RopaClone`.
const NO_CLONE: u32 = 0;
const ROPA_CLONE: u32 = 3;

/// The viewer's data and its one character.
pub(super) struct Viewer {
    data: Arc<SceneAnimData>,
    face: AndaleDatabase,
    /// `MarqueeSetting::mPos`, `mOrientation` and `mAnim`.
    position: [f32; 3],
    rotation: f32,
    anim: String,
    /// The character (a marquee, or the created skater) `LoadMarquee` 0x8281c540 loaded, its
    /// entity and renderable.
    shown: Option<(Option<u64>, SceneCharacter, SceneCharacterModels)>,
}

impl Viewer {
    /// `ViewerStartup`: the location's layout position, its orientation (degrees, layout +16)
    /// and its animation, with the animation database and the skater faces `InitFace` reads.
    pub(super) fn load(db: &AttribDb, mounts: &MountSet, location: u64) -> Result<Self, String> {
        let class = string_hash64("fe_character_location");
        let word = |index| f32::from_bits(db.layout_word(class, location, index));
        let anim = db
            .attribute(class, location, ANIMATION)
            .and_then(|attribute| attribute.text(0))
            .ok_or("character viewer animation absent")?
            .to_owned();
        let read = |path: &str| mounts.read_path(path).map_err(|e| format!("{path}: {e}"));
        Ok(Self {
            data: Arc::new(SceneAnimData::parse(read(ANIM_DB)?)?),
            face: AndaleDatabase::parse(&read(SKATER_FACE)?).map_err(|e| e.to_string())?,
            position: [word(0), word(1), word(2)],
            rotation: word(4) * f32::from_bits(0x3c8e_fa35),
            anim,
            shown: None,
        })
    }

    /// `MVManager::Update` 0x82820398 for the screen's request: another character unloads the
    /// shown one and loads it (`UpdatePendingRequests` 0x82820188), whose entity plays the clip
    /// from its start (`ViewerCharacterEntity::Update` 0x82593cd8, `PlayAnimation`); a shown one
    /// advances by `dt`. Each frame publishes the pose to its renderable.
    pub(super) fn update(
        &mut self,
        requested: Option<Option<u64>>,
        dt: f32,
        db: &AttribDb,
        mounts: &Arc<MountSet>,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
    ) -> Result<(), String> {
        match (&mut self.shown, requested) {
            (Some((shown, character, _)), Some(selection)) if *shown == selection => character.add_time(dt)?,
            (_, None) => self.shown = None,
            (_, Some(selection)) => {
                self.shown = None;
                self.shown = self.load_character(selection, db, mounts, vault, cvars)?.map(|(c, m)| (selection, c, m));
            }
        }
        if let Some((_, character, models)) = &mut self.shown {
            models.present_streamed(std::slice::from_mut(character), mounts, vault, cvars);
        }
        Ok(())
    }

    /// `LoadMarquee`: a marquee's own `MarqueeLoader` by its name; the created skater
    /// (`ViewerStartup`'s "CAC") the local player's loader (`GetManagedLoaderByCharacterID`),
    /// whose recipe the profile saved. `InitFace` looks the face up by the loader's name, then a
    /// created skater's `CAC_male` or `CAC_female`.
    fn load_character(
        &self,
        selection: Option<u64>,
        db: &AttribDb,
        mounts: &MountSet,
        vault: &Arc<AttributeRuntimeVault>,
        cvars: &InputCvarReferences,
    ) -> Result<Option<(SceneCharacter, SceneCharacterModels)>, String> {
        let (schema, category, created) = match selection {
            Some(marquee) => {
                let name = db.layout_text(CHARACTERS_MARQUEE, marquee, 0).ok_or("free skate viewer marquee name absent")?;
                (name.to_owned(), "marquee", None)
            }
            None => {
                let store = EditSkaterProfileStore::open(EditSkaterProfileStore::default_path().map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
                let Some(record) = store.load_record().map_err(|e| e.to_string())?.filter(|record| !record.data.recipe.is_empty())
                else {
                    return Ok(None);
                };
                let mut recipe = CharacterRecipe::parse(&record.data.recipe).map_err(|e| e.to_string())?;
                crate::edit_skater_character::fill_missing_parts(mounts, &mut recipe, record.data.is_male, "createacharacter")
                    .map_err(|e| e.to_string())?;
                (recipe.schema_name.clone(), "createacharacter", Some((recipe, record.data.is_male)))
            }
        };
        let mut anim = SceneCharacterAnim::viewer(Arc::clone(&self.data), &self.anim)?;
        anim.init_face(&self.face, &schema, created.as_ref().map(|(_, male)| *male))?;
        let mut character = SceneCharacter::new(0, schema, category.into(), false, self.position, [0.0; 3], anim);
        character.set_viewer_transform(self.position, self.rotation);
        character.set_time(0.0)?;
        let models = match created {
            None => SceneCharacterModels::with_clone_type(NO_CLONE),
            Some((recipe, _)) => {
                let model = recipe_model(&character, recipe, mounts, vault, cvars, ROPA_CLONE)?;
                SceneCharacterModels::with_model(&character, model, ROPA_CLONE)
            }
        };
        Ok(Some((character, models)))
    }

    /// `Shutdown`: `MVManager::Shutdown` 0x8281ea20 unloads the character.
    pub(super) fn shutdown(&mut self) {
        self.shown = None;
    }

    /// The shown character's renderable.
    pub(super) fn visuals(&self) -> Vec<SelectedSceneCharacterVisual<'_>> {
        self.shown.as_ref().map(|(_, _, models)| models.visuals()).unwrap_or_default()
    }
}
