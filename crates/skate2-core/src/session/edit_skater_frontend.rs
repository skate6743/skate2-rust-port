//! Native FrontEndState_CaC backend binding. Preview owns a temporary record;
//! only explicit selection changes the profile record.
use super::edit_skater_record::{self as record_ops, MORPH_CLASS, RecordResult};
use super::edit_skater_shop::{self as shop, EditSkaterShop, ReceiptState, ShopSelection};
use crate::{
    attribute_runtime::{AttributeRuntimeInstance, AttributeRuntimeVault},
    ui::{edit_skater::*, FeSoundRequest},
};
use skate2_formats::{
    cac_parts::CacPartDb, recipe::customizations::CharacterCustomizations,
    skater_profile::CacRecord,
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub(crate) struct EditSkaterEffects {
    pub recipe_changed: bool,
    pub finish: bool,
    pub gesture: Option<u32>,
}
pub(crate) struct EditSkaterFrontend {
    pub controller: EditSkaterController,
    pub record: CacRecord,
    db: Arc<CacPartDb>,
    vault: Arc<AttributeRuntimeVault>,
    policy: EditSkaterPolicy,
    preview: Option<CacRecord>,
    undo: CacRecord,
    parts: Vec<Option<usize>>,
    // Immutable catalog presentation only; installed choices and selection
    // remain live. Avoid GetAllPartsInWearStyleGroup for every rendered frame.
    part_rows: std::cell::RefCell<std::collections::BTreeMap<(bool, usize), EditSkaterRow>>,
    /// `mBrandFilterList`: the `sponsor` class's leaf collections, which the constructor takes
    /// from `GetLeafCollections` (`0x82680004`).
    brand_filters: Vec<u64>,
    /// The wear styles menu's `mPartList`: `mCurrentPart`'s alternates, itself first.
    wear_styles: Vec<usize>,
    /// `mCurrentWearStyleIndex`: the wear style worn, in `wear_styles`.
    wear_style_index: usize,
    /// `mCurrentPart`: the part selected or cycled to, whose wear styles menu shows.
    current_part: Option<usize>,
    stamp_zones: Vec<u64>,
    stamp_zone: Option<u64>,
    stamps: Vec<Option<StampChoice>>,
    available_parts: Vec<bool>,
    available_gestures: Vec<u32>,
    camera: u64,
    body: u64,
    gender_confirmation: Option<usize>,
    exit_confirmation: Option<usize>,
    sounds: Vec<FeSoundRequest>,
    /// `+0x12EE`: the morph slider's looping cas_slider is playing.
    slider: bool,
    /// The shop and its receipt (`mpReceiptState`).
    pub(crate) shop: EditSkaterShop,
}
const MAIN: u64 = 0xad45_4a0e_cd0e_2edf;
const SHOP: u64 = 0xb5c3_cac4_b3dc_0165;
const PETE: u64 = 0xe986_20ed_39c8_3b7a;
const STAMP_ZONE: u64 = 0xbd77_229e_4122_863e;
const STAMP: u64 = 0x0d71_da87_70e0_9cbc;
const SPONSOR: u64 = 0x548b_58b0_c27c_9564;
// RawPartDataV6: fields begin at byte28. These are serialized field indices,
// not eCACPartAttribute enum values (which include non-string attributes).
const PART_EYE_COLOUR: usize = (48 - 28) / 2;
const PART_DECK_COLOUR: usize = (38 - 28) / 2;
const PART_STYLE_SUBCATEGORY: usize = (104 - 28) / 2;
/// `PARTSLOT::SKATE_DECK`.
const SKATE_DECK: u8 = 11;

#[cfg(test)]
mod parity_tests {
    use super::*;
    #[test]
    #[ignore = "requires original SKATE2_GAME_ROOT assets"]
    fn original_editor_colour_materials_and_morphs() {
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("game root"));
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES {
            let path = spec.path_under(&root);
            if path.is_file() { mounts.push(MountedBig::open(path, spec.use_external_header).unwrap()); }
        }
        let db = Arc::new(CacPartDb::parse(&mounts.read_path(skate2_formats::cac_parts::DATABASE_PATH).unwrap()).unwrap());
        let vault = AttributeRuntimeVault::try_load_mounted(&mounts, "data/db/skater.vlt",
            &[record_ops::COLOUR_CLASS, record_ops::COLOUR_ZONE_CLASS, MORPH_CLASS, SPONSOR]).unwrap();
        for male in [true, false] {
            let record = record_ops::fresh_record(&db, &vault, male).unwrap();
            let shop = EditSkaterShop::new(&db, 0, &|_, _| true, &|_, _| false).unwrap();
            let mut frontend = EditSkaterFrontend::new(db.clone(), vault.clone(), record,
                EditSkaterPolicy { unlock_all_clothing: true }, shop, false).unwrap();
            for option in [EditSkaterOption::EyesColour, EditSkaterOption::WearStylesColour] {
                frontend.controller.category = Some(EditSkaterOption::Boards);
                let slot = frontend.colour_slot(option).unwrap();
                let part_slot = if slot == 6 { 9 } else { 11 };
                let colours = frontend.colours(option).unwrap();
                assert!(colours.len() > 1);
                for _ in 0..colours.len() { frontend.adjust_colour(option, false).unwrap(); }
                let mut materials = std::collections::BTreeSet::new();
                for (i, key) in colours.iter().enumerate() {
                    assert_eq!(frontend.record.colour_ids[slot], *key);
                    let id = frontend.record.installed_parts[part_slot];
                    let part = db.find(part_slot as u8, id.model, id.material).unwrap();
                    let name = text(&vault.instance(record_ops::COLOUR_CLASS, *key), 52).unwrap();
                    assert_eq!(db.string(part.attributes[if slot == 6 { PART_EYE_COLOUR } else { PART_DECK_COLOUR }]).unwrap(), name);
                    // Verify selection survives assembly and reaches recipe material IDs.
                    let ids = record_ops::assemble_parts(&mut frontend.record, &db, &vault, false, true).unwrap();
                    let custom = CharacterCustomizations::parse_prefix(frontend.record.customizations_bytes().unwrap()).unwrap().0;
                    let recipe = crate::edit_skater_character::compose_recipe(&db, &ids, custom).unwrap();
                    let component = recipe.components.iter().find(|c| c.name == skate2_formats::cac_parts::SLOT_NAMES[part_slot]).unwrap();
                    assert!(component.models.iter().flat_map(|m|&m.lods).flat_map(|l|&l.material_instances)
                        .flat_map(|i|&i.materials).any(|m|m.asset_id == ids[part_slot].material));
                    materials.insert(id.material);
                    frontend.adjust_colour(option, true).unwrap();
                    assert_eq!(frontend.record.colour_ids[slot], colours[(i+1).min(colours.len()-1)]);
                }
                assert!(materials.len() > 1, "{option:?}: HUD changed but material did not");
            }
        }
    }
    /// `GetSponsorLogoFileName`: a part whose brand a `sponsor` names shows that sponsor's logo,
    /// an image `screens.big` holds.
    #[test]
    #[ignore = "requires original SKATE2_GAME_ROOT assets"]
    fn original_part_brands_show_their_sponsor_logos() {
        use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
        let root = std::path::PathBuf::from(std::env::var_os("SKATE2_GAME_ROOT").expect("game root"));
        let mut mounts = MountSet::new();
        for spec in SKATE2_ARCHIVES {
            let path = spec.path_under(&root);
            if path.is_file() { mounts.push(MountedBig::open(path, spec.use_external_header).unwrap()); }
        }
        let screens = MountedBig::open(root.join("data/fe/screens.big"), false).unwrap();
        let db = Arc::new(CacPartDb::parse(&mounts.read_path(skate2_formats::cac_parts::DATABASE_PATH).unwrap()).unwrap());
        let vault = AttributeRuntimeVault::try_load_mounted(&mounts, "data/db/skater.vlt",
            &[record_ops::COLOUR_CLASS, record_ops::COLOUR_ZONE_CLASS, MORPH_CLASS, SPONSOR]).unwrap();
        let record = record_ops::fresh_record(&db, &vault, true).unwrap();
        let shop = EditSkaterShop::new(&db, 0, &|_, _| true, &|_, _| false).unwrap();
        let frontend = EditSkaterFrontend::new(db.clone(), vault, record,
            EditSkaterPolicy { unlock_all_clothing: true }, shop, false).unwrap();
        let mut logos = std::collections::BTreeSet::new();
        for index in 0..db.parts.len() {
            let Some(logo) = frontend.sponsor_logo(index).unwrap() else { continue };
            assert!(screens.read_path(&format!("data\\{logo}.rx2")).is_ok(), "{logo}");
            logos.insert(logo);
        }
        assert!(!logos.is_empty());
        assert_eq!(frontend.brand_key("not a brand").unwrap(), 0);
    }
}
#[derive(Clone)]
struct StampChoice {
    index: Option<u8>,
    path: String,
    description: String,
    material: u64,
    thumbnail: String,
}
fn word(i: &AttributeRuntimeInstance, offset: usize) -> RecordResult<u32> {
    let b = i
        .try_cached_layout_field(offset, 4)?
        .ok_or("CAC layout field missing")?;
    Ok(u32::from_be_bytes(b.bytes().try_into().unwrap()))
}
fn qword(i: &AttributeRuntimeInstance, offset: usize) -> RecordResult<u64> {
    let b = i
        .try_cached_layout_field(offset, 8)?
        .ok_or("CAC qword absent")?;
    Ok(u64::from_be_bytes(b.bytes().try_into().unwrap()))
}
fn text(i: &AttributeRuntimeInstance, offset: u32) -> RecordResult<String> {
    let b = i.try_cached_layout_text(offset)?.ok_or("CAC text absent")?;
    Ok(std::str::from_utf8(b.bytes())?
        .trim_end_matches('\0')
        .to_owned())
}
fn cycle(index: usize, n: usize, forward: bool) -> usize {
    if n == 0 {
        0
    } else if forward {
        (index + 1) % n
    } else {
        (index + n - 1) % n
    }
}
fn gesture_slot(o: EditSkaterOption) -> Option<usize> {
    use EditSkaterOption as O;
    match o {
        O::EditGestureUp => Some(0),
        O::EditGestureDown => Some(1),
        O::EditGestureLeft => Some(2),
        O::EditGestureRight => Some(3),
        _ => None,
    }
}
impl EditSkaterFrontend {
    pub(crate) fn new(
        db: Arc<CacPartDb>,
        vault: Arc<AttributeRuntimeVault>,
        record: CacRecord,
        policy: EditSkaterPolicy,
        shop: EditSkaterShop,
        new_career: bool,
    ) -> RecordResult<Self> {
        // Intentional PC QoL exception (2026-10-12): all 35 gestures are
        // always available. Keep SetScreenState(10)'s native catalog order
        // (0x82685154..0x82685f30), bypassing its category 14 unlock checks.
        let available_gestures = (0..GESTURE_UNLOCK_NAMES.len() as u32).collect();
        let mut s = Self {
            controller: EditSkaterController::default(),
            undo: record.clone(),
            record,
            preview: None,
            available_parts: db
                .parts
                .iter()
                .map(|p| p.attributes[7] == u16::MAX)
                .collect(),
            db,
            brand_filters: vault.leaf_collections_sorted(SPONSOR)?,
            vault,
            policy,
            parts: Vec::new(),
            part_rows: Default::default(),
            wear_styles: Vec::new(),
            wear_style_index: 0,
            current_part: None,
            stamp_zones: Vec::new(),
            stamp_zone: None,
            stamps: Vec::new(),
            available_gestures,
            camera: MAIN,
            body: MAIN,
            gender_confirmation: None,
            exit_confirmation: None,
            sounds: Vec::new(),
            slider: false,
            shop,
        };
        let a = s.controller.open(new_career);
        s.action(a, &mut EditSkaterEffects::default())?;
        Ok(s)
    }
    /// DLC availability is separate from the temporary clothing unlock policy.
    pub(crate) fn set_available_parts(&mut self, available: Vec<bool>) -> RecordResult<()> {
        if available.len() != self.db.parts.len() {
            return Err("CAC availability/catalog length mismatch".into());
        }
        self.available_parts = available;
        Ok(())
    }
    pub(crate) fn preview_record(&self) -> &CacRecord {
        self.preview.as_ref().unwrap_or(&self.record)
    }
    pub(crate) fn preview_record_mut(&mut self) -> &mut CacRecord {
        self.preview.as_mut().unwrap_or(&mut self.record)
    }
    pub(crate) fn camera_key(&self) -> u64 {
        self.camera
    }
    pub(crate) fn body_key(&self) -> u64 {
        self.body
    }
    /// GetStringValue 0x826775f0, PlayGestureAnimation 0x82649a98 and
    /// ReceiveInput 0x8268dd2c..0x8268e088 use index zero when the equipped
    /// gesture is absent from the available list, without rewriting the slot.
    fn gesture_index(&self, slot: usize) -> usize {
        self.available_gestures
            .iter()
            .position(|&gesture| gesture == self.record.data.gestures[slot])
            .unwrap_or(0)
    }
    /// `SetScreenState(24)` (`0x82684b40`), entering the wear styles menu or coming back to it:
    /// `mCurrentPart`'s colour when it is colourizable; unless it is a deck, its wear styles
    /// (`GetWearStyleAlternates`, which lists the part first, where `GetPartIndex` finds it) when
    /// it has others; its graphic when it is logoable.
    fn wear_styles_menu(&mut self) -> RecordResult<()> {
        use EditSkaterOption as O;
        let index = self.current_part.ok_or("CAC wear styles part absent")?;
        let part = &self.db.parts[index];
        let mut options = Vec::new();
        if flag(part.colourizable) {
            options.push(O::WearStylesColour);
        }
        self.wear_styles = Vec::new();
        if part.slot != SKATE_DECK {
            self.wear_styles = shop::wear_style_alternates(&self.db, &self.available_parts, index);
            if self.wear_styles.len() > 1 {
                self.wear_style_index = 0;
                options.push(O::WearStylesStyle);
            }
        }
        if flag(part.logoable) {
            options.push(O::WearStylesGraphic);
        }
        self.controller.set_options(options);
        Ok(())
    }
    fn gesture_presentation(&mut self) {
        self.body = self
            .controller
            .selected_option()
            .and_then(gesture_slot)
            .map(|slot| GESTURE_BODY[self.available_gestures[self.gesture_index(slot)] as usize])
            .unwrap_or(MAIN);
    }
    pub(crate) fn update(
        &mut self,
        input: EditSkaterInput,
        elapsed_ms: u32,
        preview_delay_ms: u32,
    ) -> RecordResult<EditSkaterEffects> {
        let mut e = EditSkaterEffects::default();
        if self.shop.not_enough_cash {
            // The one-option sk8popup takes A (core_a_button); NotEnoughCashCallback
            // 0x82659e78 then opens the receipt in mode 1 when it holds a part.
            if input.select {
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
                self.shop.not_enough_cash = false;
                if !self.shop.receipt().is_empty() {
                    self.shop.open_receipt(1);
                }
            }
            return Ok(e);
        }
        if self.shop.screen.is_some() {
            self.receipt_input(input, &mut e)?;
            return Ok(e);
        }
        if let Some(mut selected) = self.exit_confirmation {
            // Finish's sPopupInfo is not cancellable either: Up and Down move the wrapping
            // picker; A answers LeavingCACConfirmationCallback 0x826899d0, Yes with Shutdown,
            // No with SetScreenState(9) (the new career's mSubState 1 becomes 2, which leaves
            // its B and Start rules as they were).
            if input.up {
                selected = 1 - selected;
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_NAV_UP"));
            }
            if input.down {
                selected = 1 - selected;
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_NAV_DOWN"));
            }
            self.exit_confirmation = Some(selected);
            if input.select {
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
                self.exit_confirmation = None;
                if selected == 0 {
                    self.preview = None;
                    self.controller.close();
                    e.finish = true;
                } else {
                    let a = self.controller.push(EditSkaterMenu::EditSkaterMain);
                    self.action(a, &mut e)?;
                }
            }
            return Ok(e);
        }
        if let Some(mut selected) = self.gender_confirmation {
            // The GenderSwitch sPopupInfo is not cancellable, so sk8popup hands every key to
            // its wrapping menu_picker: Up and Down move and sound, A sounds and selects.
            if input.up {
                selected = 1 - selected;
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_NAV_UP"));
            }
            if input.down {
                selected = 1 - selected;
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_NAV_DOWN"));
            }
            self.gender_confirmation = Some(selected);
            if input.select {
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON"));
                self.gender_confirmation = None;
                if selected == 0 {
                    self.switch_gender()?;
                    e.recipe_changed = true;
                }
            }
            return Ok(e);
        }

        let previous_option = self.controller.selected_option();
        let previous_menu = self.controller.menu();
        let accepting = self.controller.visible && !self.controller.busy;
        self.controller.receipt_pending = !self.shop.receipt().is_empty();
        let actions = self.controller.update(input);
        self.sounds
            .extend(self.controller.take_sounds().into_iter().map(FeSoundRequest::Play));
        for action in actions {
            self.action(action, &mut e)?;
        }
        // createskater SetSelection, behind every style and morph Up/Down, calls
        // Game.OnMenuHilight; Push/PopScreenState call SetScreenState. Both stop the slider.
        if self.controller.menu() != previous_menu
            || accepting
                && matches!(previous_menu.movie_mode(), "style" | "morph")
                && (input.up || input.down)
        {
            self.stop_slider();
        }
        if self.controller.menu() == EditSkaterMenu::EditSkaterStyle
            && self.controller.selected_option() != previous_option
        {
            self.gesture_presentation();
        }
        if let Some(a) = self
            .controller
            .advance_preview(elapsed_ms, preview_delay_ms)
        {
            self.action(a, &mut e)?;
        }
        if self.controller.visible && !self.controller.busy {
            if let Some(o) = self
                .controller
                .selected_option()
                .filter(|o| o.definition().kind == "morph")
            {
                let settings = self
                    .vault
                    .instance(0xbb85_2b43_8bf5_78ed, 0xd7ed_bd36_2d7d_2152);
                let threshold = f32::from_bits(
                    settings
                        .try_scalar_word(0x2a50_1f56_1e08_8458, 0)?
                        .unwrap_or(0),
                );
                let step = f32::from_bits(
                    settings
                        .try_scalar_word(0x2d44_97b7_0742_fd97, 0)?
                        .unwrap_or(0),
                );
                let direction = if input.morph_axis > threshold
                    || input.morph_axis_secondary > threshold
                {
                    1.0
                } else if input.morph_axis < -threshold || input.morph_axis_secondary < -threshold {
                    -1.0
                } else {
                    0.0
                };
                // OnUpdate82670400 loops cas_slider while the stick moves the morph, and stops
                // it at a clamped bound or once the stick is back inside the threshold.
                if direction != 0.0 {
                    if self.adjust_morph(o, step * direction)? {
                        self.stop_slider();
                    } else if !self.slider {
                        self.slider = true;
                        self.sounds.push(FeSoundRequest::Loop("FE_SOUND_CAS_SLIDER"));
                    }
                    e.recipe_changed = true;
                } else {
                    self.stop_slider();
                }
            }
        }
        Ok(e)
    }
    /// The `+0x12EE` test at every cas_slider stop site.
    fn stop_slider(&mut self) {
        if std::mem::take(&mut self.slider) {
            self.sounds.push(FeSoundRequest::Stop("FE_SOUND_CAS_SLIDER"));
        }
    }
    pub(crate) fn take_sounds(&mut self) -> Vec<FeSoundRequest> {
        std::mem::take(&mut self.sounds)
    }
    /// `OnLeave` 0x82680570 stops a slider still looping after the fade.
    pub(crate) fn leave(&mut self) -> Vec<FeSoundRequest> {
        self.stop_slider();
        self.take_sounds()
    }
    fn action(&mut self, a: EditSkaterAction, e: &mut EditSkaterEffects) -> RecordResult<()> {
        use EditSkaterAction as A;
        use EditSkaterMenu as M;
        use EditSkaterOption as O;
        match a {
            A::SelectDynamicRow(row) => {
                let key = *self
                    .stamp_zones
                    .get(row)
                    .ok_or("CAC tattoo zone row out of range")?;
                self.stamp_zone = Some(key);
                let a = self.controller.push(M::TattooThumbnails);
                self.action(a, e)?;
            }
            // Finish 0x8268b828 opens the exit confirmation, default Yes; its
            // FrontEndState_Popup::OnEnter 0x82658474 sounds core_popup.
            A::Finish => {
                self.exit_confirmation = Some(0);
                self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
            }
            A::OpenReceipt => self.shop.open_receipt(0),
            A::ClearPreview => {
                e.recipe_changed |= self.preview.take().is_some();
                if self.controller.menu() != M::EditSkaterMain {
                    self.camera = SHOP;
                }
            }
            A::EnterMenu(m) => {
                match m {
                    M::Tattoos => {
                        self.stamp_zones = Vec::new();
                        for slot in [0, 1, 23] {
                            self.stamp_zones.extend(self.zones_for_slot(slot, true)?);
                        }
                        self.controller.set_dynamic_count(self.stamp_zones.len());
                    }
                    M::TattooThumbnails | M::GraphicThumbnails => {
                        self.load_stamps()?;
                        self.controller.set_part_count(self.stamps.len(), None);
                        if m == M::TattooThumbnails {
                            let i = self.vault.instance(
                                STAMP_ZONE,
                                self.stamp_zone.ok_or("CAC stamp zone absent")?,
                            );
                            self.camera = qword(&i, 0)?;
                        }
                    }
                    M::EditSkaterMain => {
                        self.camera = MAIN;
                        self.body = MAIN;
                        self.undo = self.record.clone();
                    }
                    M::Slappys => {
                        self.camera = SHOP;
                        self.body = SHOP;
                    }
                    M::EditSkaterStyle => self.gesture_presentation(),
                    M::WearStyles => self.wear_styles_menu()?,
                    M::BackAlleyPete => {
                        self.camera = PETE;
                        self.body = PETE;
                        if !self.record.data.is_male {
                            self.controller.set_options(vec![
                                O::Gender,
                                O::Skin,
                                O::BodyShape,
                                O::Hair,
                                O::FaceMod,
                            ]);
                        }
                    }
                    _ => {}
                }
                if let Some(o) = menu_option(m) {
                    if let Some(key) = o.camera_key() {
                        self.camera = key;
                    }
                }
            }
            A::LoadParts(o) => {
                let indices = clothing_parts(&self.db, o, self.record.data.is_male, |p| {
                    self.db
                        .parts
                        .iter()
                        .position(|q| std::ptr::eq(p, q))
                        .is_some_and(|i| self.available_parts[i])
                })?;
                self.parts = indices.into_iter().map(Some).collect();
                let slot = clothing_slot(o).ok_or("CAC category has no native part slot")?;
                let removable = record_ops::can_remove_part(slot);
                if removable {
                    self.parts.insert(0, None);
                }
                let worn = self.parts.iter().position(|p| match p {
                    Some(i) => {
                        let p = &self.db.parts[*i];
                        let id = self.record.installed_parts[slot as usize];
                        p.model_id == id.model && p.material_id == id.material
                    }
                    None => {
                        self.record.installed_parts[slot as usize]
                            == skate2_formats::skater_profile::CacPartId::invalid()
                    }
                });
                // SetScreenState82684b40: nullable slots start on Remove;
                // other categories put the worn part at the front.
                if !removable {
                    if let Some(index) = worn {
                        let worn = self.parts.remove(index);
                        self.parts.insert(0, worn);
                    }
                }
                self.controller.set_part_count(self.parts.len(), Some(0));
                if let Some(key) = o.camera_key() {
                    self.camera = key;
                }
            }
            A::PreviewPart(row) => {
                if let Some(i) = *self.parts.get(row).ok_or("CAC preview row out of range")? {
                    let mut preview = self.record.clone();
                    record_ops::wear_part(&mut preview, &self.db, &self.db.parts[i])?;
                    self.preview = Some(preview);
                } else {
                    self.preview = None;
                }
                e.recipe_changed = true;
            }
            A::SelectPart(row) => {
                if matches!(
                    self.controller.menu(),
                    M::TattooThumbnails | M::GraphicThumbnails
                ) {
                    self.select_stamp(row)?;
                    e.recipe_changed = true;
                    return Ok(());
                }
                let p = *self.parts.get(row).ok_or("CAC selected row out of range")?;
                // OnThumbnailSelect8268aee0: the remove row, an owned part and one already on
                // the receipt sound core_a_button; a locked part, or one the balance does not
                // cover (whose popup's FrontEndState_Popup::OnEnter82658474 sounds core_popup),
                // sounds core_error and stays off; a new part joins the receipt (cas_accept)
                // before it is worn.
                let verdict = match p {
                    Some(i) if !self.policy.unlock_all_clothing => self.shop.select(&self.db, &self.record, i),
                    _ => ShopSelection::Wear,
                };
                match verdict {
                    ShopSelection::Locked => {
                        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_ERROR"));
                        return Ok(());
                    }
                    ShopSelection::NotEnoughCash => {
                        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_ERROR"));
                        self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
                        return Ok(());
                    }
                    ShopSelection::AddedToReceipt => self.sounds.push(FeSoundRequest::Play("FE_SOUND_CAS_ACCEPT")),
                    ShopSelection::Wear => self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_A_BUTTON")),
                }
                self.preview = None;
                if let Some(i) = p {
                    // A colourizable or logoable part, or one with other wear styles, opens its
                    // wear styles menu; a deck of the worn deck's style subcategory leaves the worn
                    // deck on (worn again), and is not worn when it is that deck.
                    let db = Arc::clone(&self.db);
                    let part = &db.parts[i];
                    let mut wear = Some(part);
                    let alternates = shop::wear_style_alternates(&db, &self.available_parts, i).len();
                    if flag(part.colourizable) || alternates > 1 || flag(part.logoable) {
                        self.current_part = Some(i);
                        let a = self.controller.push(M::WearStyles);
                        self.action(a, e)?;
                        let style = part.attributes[PART_STYLE_SUBCATEGORY];
                        let worn = self.record.installed_parts[usize::from(SKATE_DECK)];
                        if part.slot == SKATE_DECK
                            && style != u16::MAX
                            && let Some(deck) = db.find(SKATE_DECK, worn.model, worn.material)
                            && deck.attributes[PART_STYLE_SUBCATEGORY] == style
                        {
                            wear = (deck.model_id != part.model_id || deck.material_id != part.material_id).then_some(deck);
                        }
                    }
                    if let Some(part) = wear {
                        record_ops::wear_part(&mut self.record, &db, part)?;
                    }
                } else {
                    record_ops::remove_part(
                        &mut self.record,
                        &self.db,
                        &self.undo,
                        clothing_slot(
                            self.controller
                                .category
                                .ok_or("CAC remove category missing")?,
                        )
                        .ok_or("CAC remove slot missing")?,
                    )?;
                }
                e.recipe_changed = true;
            }
            A::AdjustOption { option, forward } => {
                if self.adjust(option, forward, e)? {
                    e.recipe_changed = true;
                }
            }
            A::ActivateOption(o) => match o {
                O::WearStylesGraphic => {
                    let slot = clothing_slot(
                        self.controller
                            .category
                            .ok_or("CAC graphic category absent")?,
                    )
                    .ok_or("CAC graphic slot absent")?;
                    let zones = self.zones_for_slot(slot, false)?;
                    if zones.len() != 1 {
                        return Err("CAC garment requires exactly one stamp zone".into());
                    }
                    self.stamp_zone = Some(zones[0]);
                    let a = self.controller.push(M::GraphicThumbnails);
                    self.action(a, e)?;
                }
                O::Finished => self.action(A::Finish, e)?,
                O::WearStylesAccept => {
                    for a in self.controller.update(EditSkaterInput {
                        back: true,
                        ..Default::default()
                    }) {
                        self.action(a, e)?;
                    }
                }
                O::PartUndo => {
                    self.record = self.undo.clone();
                    self.preview = None;
                    e.recipe_changed = true;
                }
                O::MorphingUndo => {
                    let (mut custom, _) =
                        CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
                    let (undo, _) =
                        CharacterCustomizations::parse_prefix(self.undo.customizations_bytes()?)?;
                    custom.morph_bits = undo.morph_bits;
                    self.record.replace_customizations(&custom.encode()?)?;
                    e.recipe_changed = true;
                }
                O::Gender => {
                    // OnMenuSelect8268b408 opens the GenderSwitch popup, whose
                    // FrontEndState_Popup::OnEnter82658474 sounds core_popup.
                    self.gender_confirmation = Some(0);
                    self.sounds.push(FeSoundRequest::Play("FE_SOUND_CORE_POPUP"));
                }
                _ if o.definition().kind != "option" => {}
                _ => {
                    return Err(
                        format!("CAC native option {o:?} needs its operation binding").into(),
                    );
                }
            },
        }
        Ok(())
    }
    /// `FrontEndState_Receipt::ReceiveInput` (`0x82680e08`) and `OnMenuSelect`
    /// (`0x82680ef0`) behind `cas/receipt`'s `UpdateInput`: the menu picker (no wrap) takes
    /// Purchase, Edit Cart or Back to Store; the list moves with Up and Down and toggles its
    /// line with X; B leaves the list for the picker (mode 0) or keeps the cart (mode 1).
    fn receipt_input(&mut self, input: EditSkaterInput, e: &mut EditSkaterEffects) -> RecordResult<()> {
        let receipt = self.shop.receipt().to_vec();
        let Some(screen) = self.shop.screen.as_mut() else { return Ok(()) };
        let play = |sounds: &mut Vec<FeSoundRequest>, name| sounds.push(FeSoundRequest::Play(name));
        match screen.state {
            ReceiptState::MenuPicker => {
                if input.up && screen.option > 0 {
                    screen.option -= 1;
                    play(&mut self.sounds, "FE_SOUND_CORE_NAV_UP");
                }
                if input.down && screen.option + 1 < RECEIPT_OPTIONS.len() {
                    screen.option += 1;
                    play(&mut self.sounds, "FE_SOUND_CORE_NAV_DOWN");
                }
                if !input.select {
                    return Ok(());
                }
                play(&mut self.sounds, "FE_SOUND_CORE_A_BUTTON");
                let (option, kept) = (screen.option, screen.kept.clone());
                match option {
                    0 => {
                        play(&mut self.sounds, "FE_SOUND_CAS_PUCHASE_ITEM");
                        self.shop.update_receipt_list(&self.db, &self.available_parts, &mut self.record, &self.undo, &kept)?;
                        self.shop.purchase(&self.db, &self.available_parts, &mut self.record)?;
                        self.close_receipt(0, e)?;
                    }
                    1 => screen.state = ReceiptState::ReceiptList,
                    _ => {
                        self.shop.update_receipt_list(&self.db, &self.available_parts, &mut self.record, &self.undo, &kept)?;
                        self.close_receipt(2, e)?;
                    }
                }
            }
            ReceiptState::ReceiptList => {
                if input.up && screen.row > 0 {
                    screen.row -= 1;
                    play(&mut self.sounds, "FE_SOUND_CORE_NAV_UP");
                }
                if input.down && screen.row + 1 < receipt.len() {
                    screen.row += 1;
                    play(&mut self.sounds, "FE_SOUND_CORE_NAV_DOWN");
                }
                if input.misc && let Some(&line) = receipt.get(screen.row) {
                    play(&mut self.sounds, "FE_SOUND_CORE_A_BUTTON");
                    if let Some(at) = screen.kept.iter().position(|&kept| kept == line) {
                        screen.kept.remove(at);
                    } else {
                        screen.kept.push(line);
                    }
                }
                if input.back {
                    play(&mut self.sounds, "FE_SOUND_CORE_B_BUTTON");
                    if screen.mode == 1 {
                        let kept = screen.kept.clone();
                        self.shop.update_receipt_list(&self.db, &self.available_parts, &mut self.record, &self.undo, &kept)?;
                        self.close_receipt(-1, e)?;
                    } else {
                        screen.state = ReceiptState::MenuPicker;
                    }
                }
            }
        }
        Ok(())
    }
    /// `OnCompleteImpl` (`0x82659fa8`) pops the receipt; `FrontEndState_CaC::OnGainFocus`
    /// (`0x8268ad88`) then opens the Edit Skater menu after a purchase.
    fn close_receipt(&mut self, last_option: i32, e: &mut EditSkaterEffects) -> RecordResult<()> {
        self.shop.screen = None;
        self.shop.last_option = last_option;
        self.preview = None;
        e.recipe_changed = true;
        if last_option == 0 {
            let a = self.controller.push(EditSkaterMenu::EditSkaterMain);
            self.action(a, e)?;
        }
        Ok(())
    }
    /// What `cas/receipt` reads of the pushed receipt.
    fn receipt_view(&self) -> RecordResult<Option<ReceiptView>> {
        let Some(screen) = &self.shop.screen else { return Ok(None) };
        let mut lines = Vec::new();
        for &line in self.shop.receipt() {
            let part = &self.db.parts[line];
            lines.push(ReceiptLine {
                component: hal("ID_CAC_COMPONENT_TYPE_", skate2_formats::cac_parts::SLOT_NAMES[part.slot as usize]),
                description_candidates: self.description_candidates(line)?,
                price: shop::cost(part),
                kept: screen.kept.contains(&line),
            });
        }
        let subtotal: i32 = screen.kept.iter().map(|&line| shop::cost(&self.db.parts[line])).sum();
        Ok(Some(ReceiptView {
            list: screen.state == ReceiptState::ReceiptList,
            option: screen.option,
            row: screen.row,
            lines,
            cash: self.shop.cash(),
            subtotal,
            balance: self.shop.cash() - subtotal,
        }))
    }
    fn switch_gender(&mut self) -> RecordResult<()> {
        use EditSkaterOption as O;

        record_ops::switch_gender(&mut self.record, &self.db, &self.vault)?;
        self.undo = self.record.clone();

        self.controller.set_options(if self.record.data.is_male {
            vec![
                O::Gender,
                O::Skin,
                O::BodyShape,
                O::Hair,
                O::FacialHair,
                O::FaceMod,
            ]
        } else {
            vec![O::Gender, O::Skin, O::BodyShape, O::Hair, O::FaceMod]
        });

        Ok(())
    }
    fn morph_zone(&self, o: EditSkaterOption) -> RecordResult<usize> {
        let key = o.morph_key().ok_or("CAC morph collection missing")?;
        Ok(word(&self.vault.instance(MORPH_CLASS, key), 8)? as usize)
    }
    /// True when the step passed a bound and was clamped.
    fn adjust_morph(&mut self, o: EditSkaterOption, delta: f32) -> RecordResult<bool> {
        let zone = self.morph_zone(o)?;
        let (min, max) = self.morph_limits(zone)?;
        let (mut c, _) =
            CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        let value = c
            .morph_bits
            .get_mut(zone)
            .ok_or("CAC morph zone out of range")?;
        let stepped = (f32::from_bits(*value).clamp(min, max) - min) / (max - min) + delta;
        let normalized = stepped.clamp(0.0, 1.0);
        *value = ((max - min) * normalized + min).to_bits();
        self.record.replace_customizations(&c.encode()?)?;
        Ok(stepped > 1.0 || stepped < 0.0)
    }
    // CASMorphParams::Init825916c8; frontend controls normalized values while
    // CACRecord stores the interpolated authored male/female morph range.
    fn morph_limits(&self, zone: usize) -> RecordResult<(f32, f32)> {
        let offset = if self.record.data.is_male { 12 } else { 24 };
        let mut fallback = None;
        let mut exact = None;
        for key in self.vault.collections_sorted(MORPH_CLASS)? {
            let i = self.vault.instance(MORPH_CLASS, key);
            let z = word(&i, 4)? as usize;
            let bounds = (
                f32::from_bits(word(&i, offset)?),
                f32::from_bits(word(&i, offset + 4)?),
            );
            if z > 20 {
                fallback = Some(bounds);
            } else if z == zone {
                exact = Some(bounds);
            }
        }
        let bounds = exact.or(fallback).ok_or("CAC morph limits absent")?;
        if bounds.0 >= bounds.1 {
            return Err("CAC morph range invalid".into());
        }
        Ok(bounds)
    }
    fn adjust(
        &mut self,
        o: EditSkaterOption,
        forward: bool,
        e: &mut EditSkaterEffects,
    ) -> RecordResult<bool> {
        use EditSkaterOption as O;
        if o.definition().kind == "color" {
            self.adjust_colour(o, forward)?;
            return Ok(true);
        }
        if let Some(slot) = gesture_slot(o) {
            let i = self.gesture_index(slot);
            let id = self.available_gestures[cycle(i, self.available_gestures.len(), forward)];
            self.record.data.gestures[slot] = id;
            self.body = GESTURE_BODY[id as usize];
            e.gesture = Some(id);
            return Ok(true);
        }
        match o {
            O::EditTrucks | O::EditWheels => {
                let i = usize::from(o == O::EditWheels);
                let v = f32::from_bits(self.record.data.tuning_bits[i]);
                self.record.data.tuning_bits[i] = (v + if forward { 0.1 } else { -0.1 })
                    .clamp(0.0, 1.0)
                    .to_bits();
            }
            O::EditStance => self.record.data.stance = u32::from(self.record.data.stance == 0),
            O::EditPosture => {
                self.record.data.style = cycle(self.record.data.style as usize, 3, forward) as u32
            }
            O::HairStyle | O::FacialHairStyle | O::BrowStyle => {
                let parts = self.body_parts(o)?;
                let slot = if o == O::HairStyle { 2 } else { 3 };
                let id = self.record.installed_parts[slot];
                let current = parts
                    .iter()
                    .position(|p| match p {
                        None => id == skate2_formats::skater_profile::CacPartId::invalid(),
                        Some(i) => {
                            let p = &self.db.parts[*i];
                            p.model_id == id.model && p.material_id == id.material
                        }
                    })
                    .unwrap_or(0);
                match parts
                    .get(cycle(current, parts.len(), forward))
                    .ok_or("CAC body selector empty")?
                {
                    Some(i) => {
                        record_ops::wear_part(&mut self.record, &self.db, &self.db.parts[*i])?
                    }
                    None => record_ops::remove_part(&mut self.record, &self.db, &self.undo, 2)?,
                }
            }
            // ReceiveInput 0x8268d818: the next wear style, wrapping, becomes mCurrentPart and is
            // worn.
            O::WearStylesStyle => {
                if self.wear_styles.is_empty() {
                    return Err("CAC wear style list empty".into());
                }
                self.wear_style_index = cycle(self.wear_style_index, self.wear_styles.len(), forward);
                let index = self.wear_styles[self.wear_style_index];
                self.current_part = Some(index);
                record_ops::wear_part(&mut self.record, &self.db, &self.db.parts[index])?;
            }
            _ if o.definition().kind == "morph"
                || o.definition().kind == "option"
                || o.definition().kind == "category"
                || o.definition().kind == "list" =>
            {
                return Ok(false);
            }
            _ => {
                return Err(format!(
                    "CAC native adjustable option {o:?} needs its operation binding"
                )
                .into());
            }
        }
        Ok(true)
    }
    fn body_parts(&self, o: EditSkaterOption) -> RecordResult<Vec<Option<usize>>> {
        use EditSkaterOption as O;
        let mut parts = Vec::new();
        let id = self.record.installed_parts[3];
        let base = self
            .db
            .find(3, id.model, id.material)
            .ok_or("CAC rostral absent")?;
        for (i, p) in self.db.parts.iter().enumerate() {
            if !self.available_parts[i] || flag(p.used_in_game) {
                continue;
            }
            let gender = self.db.attribute(p, 15)?;
            if gender
                == if self.record.data.is_male {
                    "female"
                } else {
                    "male"
                }
            {
                continue;
            }
            let accept = if o == O::HairStyle {
                p.slot == 2 && self.db.string(p.attributes[16])? == "full"
            } else {
                p.slot == 3
                    && [13, 35, 14, 15]
                        .iter()
                        .all(|&a| p.attributes[a] == base.attributes[a])
                    && if o == O::FacialHairStyle {
                        p.attributes[9] == base.attributes[9]
                    } else {
                        p.attributes[12] == base.attributes[12]
                            && (!self.record.data.is_male
                                || self.db.string(p.attributes[9])? != "style7")
                    }
            };
            if accept {
                parts.push(i);
            }
        }
        sort_clothing_parts(&self.db, &mut parts)?;
        if o == O::BrowStyle {
            crate::native_execution_sort::sort(&mut parts, &mut |a, b| {
                Ok::<_, skate2_formats::cac_parts::CacPartError>(
                    self.db.string(self.db.parts[a].attributes[9])?.as_bytes()
                        < self.db.string(self.db.parts[b].attributes[9])?.as_bytes(),
                )
            })?;
        }
        let mut parts: Vec<_> = parts.into_iter().map(Some).collect();
        if o == O::HairStyle && self.record.data.is_male {
            parts.insert(0, None);
        }
        Ok(parts)
    }
    fn colour_slot(&self, o: EditSkaterOption) -> RecordResult<usize> {
        use EditSkaterOption as O;
        Ok(match o {
            O::HairColour => 1,
            O::FacialHairColour => 7,
            O::EyesColour => 6,
            O::SkinColourDarkness => 2,
            O::WearStylesColour => match clothing_slot(
                self.controller
                    .category
                    .ok_or("CAC colour category absent")?,
            )
            .ok_or("CAC colour part slot absent")?
            {
                18 => 0,
                2 => 1,
                0 | 1 | 3 => 2,
                9 => 6,
                23 => 9,
                17 => 10,
                11 => 11,
                _ => return Err("CAC part has no colour zone".into()),
            },
            _ => return Err("CAC option has no colour zone".into()),
        })
    }
    fn zones_for_slot(&self, slot: u8, skinned: bool) -> RecordResult<Vec<u64>> {
        let mut out = Vec::new();
        for key in self.vault.collections_sorted(STAMP_ZONE)? {
            let i = self.vault.instance(STAMP_ZONE, key);
            let Some(component) = i.try_cached_layout_text(16)? else {
                continue;
            };
            let name = std::str::from_utf8(component.bytes())?.trim_end_matches('\0');
            let skin = i
                .try_cached_layout_field(32, 1)?
                .ok_or("CAC stamp zone skin flag absent")?
                .bytes()[0]
                != 0;
            if name == skate2_formats::cac_parts::SLOT_NAMES[slot as usize] && skin == skinned {
                out.push(key);
            }
        }
        Ok(out)
    }
    fn load_stamps(&mut self) -> RecordResult<()> {
        let zone = self
            .vault
            .instance(STAMP_ZONE, self.stamp_zone.ok_or("CAC stamp zone missing")?);
        let index = word(&zone, 28)?;
        self.stamps = vec![None];
        if (6..=8).contains(&index) {
            for p in self.db.parts_in_slot(10) {
                let path = self.db.attribute(p, 36)?;
                // RemovePlaceholderStamps825931c8 default-enabled cvar.
                if path.contains("logoEmpty") {
                    continue;
                }
                self.stamps.push(Some(StampChoice {
                    index: None,
                    path: path.into(),
                    description: "ID_CUSTOM_LOGO".into(),
                    material: p.material_id,
                    thumbnail: format!("fe\\cas\\logos\\logo_object_{path}").to_ascii_lowercase(),
                }));
            }
        } else {
            let field = 0xe7ac_d351_b393_7f72;
            for index in 0..zone.try_length(field)? {
                let data = zone
                    .try_array_field_data(field, index, 0x761c_0328_0f99_4647, 16, 0..8)?
                    .ok_or("CAC stamp reference missing")?;
                let key = u64::from_be_bytes(data.bytes().try_into().unwrap());
                let stamp = self.vault.instance(STAMP, key);
                let index = u8::try_from(word(&stamp, 8)?)
                    .map_err(|_| "CAC embedded stamp index overflow")?;
                self.stamps.push(Some(StampChoice {
                    index: Some(index),
                    path: text(&stamp, 0)?,
                    description: text(&stamp, 4)?,
                    material: 0,
                    thumbnail: format!("fe\\cas\\tattoo\\tattoo{}", u32::from(index) + 1),
                }));
            }
        }
        Ok(())
    }
    fn select_stamp(&mut self, row: usize) -> RecordResult<()> {
        let stamp = self.stamps.get(row).ok_or("CAC stamp row out of range")?;
        let zone = self
            .vault
            .instance(STAMP_ZONE, self.stamp_zone.ok_or("CAC stamp zone absent")?);
        let index = u8::try_from(word(&zone, 28)?).map_err(|_| "CAC stamp zone index overflow")?;
        let (mut c, _) =
            CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        if index < 6 {
            let value = stamp.as_ref().and_then(|s| s.index).unwrap_or(8);
            let pair = c
                .embedded
                .iter_mut()
                .find(|p| p[0] == index)
                .ok_or("CAC tattoo zone absent")?;
            pair[1] = value;
        } else {
            let name = text(&zone, 16)?;
            let slot = skate2_formats::cac_parts::SLOT_NAMES
                .iter()
                .position(|&s| s == name)
                .ok_or("CAC stamp component missing")?;
            let key = match slot {
                18 => 0,
                23 => 1,
                11 => 2,
                _ => return Err("CAC dynamic stamp component invalid".into()),
            };
            c.stamps.retain(|s| s.key != key);
            if let Some(stamp) = stamp {
                let target = self.record.installed_parts[slot];
                c.stamps
                    .push(skate2_formats::recipe::customizations::TextureStamp {
                        path: stamp.path.clone(),
                        model_id: target.model,
                        material_id: target.material,
                        stamp_material_id: stamp.material,
                        user_generated: false,
                        key,
                    });
            }
        }
        self.record.replace_customizations(&c.encode()?)?;
        Ok(())
    }
    fn colours(&self, o: EditSkaterOption) -> RecordResult<Vec<u64>> {
        let slot = self.colour_slot(o)?;
        let mut selected = None;
        for key in self
            .vault
            .collections_sorted(record_ops::COLOUR_ZONE_CLASS)?
        {
            let i = self.vault.instance(record_ops::COLOUR_ZONE_CLASS, key);
            if word(&i, 44)? as usize == slot {
                selected = Some(i);
            }
        }
        let i = selected.ok_or("CAC colour zone absent")?;
        let data = i
            .try_cached_layout_field(8, 8)?
            .ok_or("CAC colour category absent")?;
        let seed = u64::from_be_bytes(data.bytes().try_into().unwrap());
        Ok(self
            .vault
            .descendants_sorted(record_ops::COLOUR_CLASS, seed)?)
    }
    fn adjust_colour(&mut self, o: EditSkaterOption, forward: bool) -> RecordResult<()> {
        let slot = self.colour_slot(o)?;
        let colours = self.colours(o)?;
        let at = colours
            .iter()
            .position(|&key| key == self.record.colour_ids[slot])
            .unwrap_or(0);
        // FrontEndState_CaC::OnKey clamps colour selection at either end.
        // Unlike a style selector, a colour strip does not wrap around.
        let next = if forward { at.saturating_add(1).min(colours.len().saturating_sub(1)) }
            else { at.saturating_sub(1) };
        let key = *colours
            .get(next)
            .ok_or("CAC colour list empty")?;
        let colour = self.vault.instance(record_ops::COLOUR_CLASS, key);
        let base = [word(&colour, 32)?, word(&colour, 36)?, word(&colour, 40)?];
        let highlight = [word(&colour, 0)?, word(&colour, 4)?, word(&colour, 8)?];
        let (mut custom, _) =
            CharacterCustomizations::parse_prefix(self.record.customizations_bytes()?)?;
        let slots: &[usize] = match slot {
            1 => &[1, 8],
            2 => &[2, 3, 4, 5],
            _ => std::slice::from_ref(&slot),
        };
        for &slot in slots {
            self.record.colour_ids[slot] = key;
            let c = custom
                .colours
                .iter_mut()
                .find(|c| c.key == slot as i32)
                .ok_or("CAC colour slot missing")?;
            c.base_rgb_bits = base;
            c.highlight_rgb_bits = highlight;
        }
        if slot == 6 || slot == 11 {
            let name = colour
                .try_cached_layout_text(52)?
                .ok_or("CAC colour name missing")?;
            let name = std::str::from_utf8(name.bytes())?.trim_end_matches('\0');
            let value = self
                .db
                .strings
                .iter()
                .position(|s| s == name)
                .and_then(|n| u16::try_from(n).ok())
                .unwrap_or(u16::MAX);
            let target = if slot == 6 {
                let id = self.record.installed_parts[9];
                let old = self
                    .db
                    .find(9, id.model, id.material)
                    .ok_or("CAC eyes absent")?;
                if old.attributes[PART_EYE_COLOUR] == value { Some(old) } else { self.db
                    .parts_in_slot(9)
                    .find(|p| p.attributes[13] == old.attributes[13] && p.attributes[PART_EYE_COLOUR] == value) }
            } else {
                self.db
                    .parts_in_slot(11)
                    .find(|p| !flag(p.used_in_game) && flag(p.logoable) && p.attributes[PART_DECK_COLOUR] == value)
            };
            if let Some(p) = target {
                if slot == 6 {
                    self.record.installed_parts[9] = skate2_formats::skater_profile::CacPartId {
                        model: p.model_id,
                        material: p.material_id,
                    };
                } else {
                    record_ops::wear_part(&mut self.record, &self.db, p)?;
                }
            }
        }
        // SetColourForZone82599438 binds to the currently installed part.
        // Keeping an earlier item's IDs makes e.g. a newly equipped sock
        // display the new HUD colour while its shader retains the old colour.
        let target_slot = match slot {
            0 => Some(18), 1 => Some(2), 6 => Some(9),
            9 => Some(23), 10 => Some(17), _ => None,
        };
        if let Some(target_slot) = target_slot {
            let id = self.record.installed_parts[target_slot];
            let colour = custom.colours.iter_mut().find(|c| c.key == slot as i32)
                .ok_or("CAC colour slot missing")?;
            colour.model_id = id.model;
            colour.material_id = id.material;
        }
        self.record.replace_customizations(&custom.encode()?)?;
        Ok(())
    }
    pub(crate) fn view(&self) -> RecordResult<EditSkaterView> {
        let menu = self.controller.menu();
        let mut rows = Vec::new();
        if menu == EditSkaterMenu::Tattoos {
            for &key in &self.stamp_zones {
                let i = self.vault.instance(STAMP_ZONE, key);
                rows.push(EditSkaterRow {
                    title: text(&i, 24)?,
                    kind: "list".into(),
                    ..Default::default()
                });
            }
        } else if matches!(
            menu,
            EditSkaterMenu::TattooThumbnails | EditSkaterMenu::GraphicThumbnails
        ) {
            for stamp in &self.stamps {
                let description = if let Some(s) = stamp {
                    if menu == EditSkaterMenu::GraphicThumbnails {
                        "ID_CUSTOM_LOGO".into()
                    } else {
                        s.description.clone()
                    }
                } else {
                    if menu == EditSkaterMenu::GraphicThumbnails {
                        "ID_CAC_REMOVE_STICKER"
                    } else {
                        "ID_CAC_REMOVE_TATTOO"
                    }
                    .into()
                };
                rows.push(EditSkaterRow {
                    title: description.clone(),
                    description,
                    texture: Some(
                        stamp
                            .as_ref()
                            .map(|s| s.thumbnail.clone())
                            .unwrap_or_else(|| "fe\\cas\\general\\remove".into()),
                    ),
                    status: "owned".into(),
                    ..Default::default()
                });
            }
        } else if menu == EditSkaterMenu::PartThumbnails {
            for entry in &self.parts {
                let row = if let Some(i) = entry {
                    let key = (self.record.data.is_male, *i);
                    if let Some(row) = self.part_rows.borrow().get(&key) {
                        rows.push(EditSkaterRow { status: self.part_status(*i).into(), is_new: self.shop.is_new(*i), ..row.clone() });
                        continue;
                    }
                    let p = &self.db.parts[*i];
                    let row = EditSkaterRow {
                        description_candidates: self.description_candidates(*i)?,
                        title: self.part_description(*i)?,
                        description: self.part_description(*i)?,
                        texture: Some(format!(
                            "fe\\cas\\{}",
                            clothing_thumbnail(&self.db, p, self.record.data.is_male)?
                        )),
                        sponsor_logo: self.sponsor_logo(*i)?,
                        status: self.part_status(*i).into(),
                        is_new: self.shop.is_new(*i),
                        cost: shop::cost(p),
                        ..Default::default()
                    };
                    self.part_rows.borrow_mut().insert(key, row.clone());
                    row
                } else {
                    EditSkaterRow {
                        title: "ID_CAC_REMOVE_PART".into(),
                        description: "ID_CAC_REMOVE_PART".into(),
                        texture: Some("fe\\cas\\general\\remove".into()),
                        status: "owned".into(),
                        ..Default::default()
                    }
                };
                rows.push(row);
            }
        } else {
            for &o in self.controller.options() {
                let d = o.definition();
                let mut row = EditSkaterRow {
                    title: d.title.into(),
                    description: d.description.into(),
                    kind: d.kind.into(),
                    ..Default::default()
                };
                if let Some(slot) = gesture_slot(o) {
                    let gesture = self.available_gestures[self.gesture_index(slot)];
                    row.subtitle = GESTURE_HAL[gesture as usize].into();
                }
                if d.kind == "color" {
                    let colours = self.colours(o)?;
                    row.value = colours
                        .iter()
                        .position(|&k| k == self.record.colour_ids[self.colour_slot(o).unwrap()])
                        .unwrap_or(0) as i32;
                    for key in colours {
                        let c = self.vault.instance(record_ops::COLOUR_CLASS, key);
                        row.colours.push(
                            [16, 20, 24]
                                .map(|offset| {
                                    word(&c, offset).map(|v| (f32::from_bits(v) * 255.0) as u8)
                                })
                                .into_iter()
                                .collect::<RecordResult<Vec<_>>>()?
                                .try_into()
                                .unwrap(),
                        );
                    }
                }
                match o {
                    EditSkaterOption::HairStyle
                    | EditSkaterOption::FacialHairStyle
                    | EditSkaterOption::BrowStyle => {
                        let slot = if o == EditSkaterOption::HairStyle {
                            2
                        } else {
                            3
                        };
                        let id = self.record.installed_parts[slot];
                        if let Some(p) = self.db.find(slot as u8, id.model, id.material) {
                            let (prefix, attr) = match o {
                                EditSkaterOption::HairStyle => ("ID_CAC_HAIRSTYLE_", 19),
                                EditSkaterOption::FacialHairStyle => ("ID_CAC_FACIAL_HAIR_", 14),
                                _ => ("ID_CAC_EYEBROW_", 11),
                            };
                            row.subtitle = hal(prefix, self.db.attribute(p, attr)?);
                        } else {
                            row.subtitle = "ID_CAC_HAIRSTYLE_BALD".into();
                        }
                    }
                    // GetStringValue 0x826775f0.
                    EditSkaterOption::WearStylesStyle => {
                        row.subtitle = format!("ID_CAC_WEAR_STYLE_OPTION_{}", self.wear_style_index + 1);
                    }
                    EditSkaterOption::Gender => {
                        row.title = if self.record.data.is_male {
                            "ID_CAC_CHOOSE_GENDER_MALE"
                        } else {
                            "ID_CAC_CHOOSE_GENDER_FEMALE"
                        }
                        .into()
                    }
                    EditSkaterOption::EditTrucks | EditSkaterOption::EditWheels => {
                        let wheel = o == EditSkaterOption::EditWheels;
                        let v = f32::from_bits(self.record.data.tuning_bits[usize::from(wheel)]);
                        row.value = (v * 10.0 + 0.5).floor() as i32;
                        row.subtitle = if wheel {
                            if v < 0.35 {
                                "ID_CAC_EDIT_WHEELS_SOFT"
                            } else if v <= 0.75 {
                                "ID_CAC_EDIT_WHEELS_MEDIUM"
                            } else {
                                "ID_CAC_EDIT_WHEELS_HARD"
                            }
                        } else {
                            if v < 0.35 {
                                "ID_CAC_EDIT_TRUCKS_LOOSE"
                            } else if v <= 0.75 {
                                "ID_CAC_EDIT_TRUCKS_MEDIUM"
                            } else {
                                "ID_CAC_EDIT_TRUCKS_TIGHT"
                            }
                        }
                        .into();
                    }
                    EditSkaterOption::EditStance => {
                        row.subtitle = if self.record.data.stance == 0 {
                            "ID_CAC_STANCE_GOOFY"
                        } else {
                            "ID_CAC_STANCE_REGULAR"
                        }
                        .into()
                    }
                    _ if d.kind == "morph" => {
                        let zone = self.morph_zone(o)?;
                        let (min, max) = self.morph_limits(zone)?;
                        let (c, _) = CharacterCustomizations::parse_prefix(
                            self.record.customizations_bytes()?,
                        )?;
                        row.value = ((f32::from_bits(
                            *c.morph_bits
                                .get(zone)
                                .ok_or("CAC morph zone out of range")?,
                        )
                        .clamp(min, max)
                            - min)
                            / (max - min)
                            * 100.0
                            + 0.5)
                            .floor() as i32;
                    }
                    _ => {}
                }
                rows.push(row);
            }
        }
        let help_a = if matches!(menu.movie_mode(), "list" | "thumbs")
            || self
                .controller
                .selected_option()
                .is_some_and(|o| o.definition().kind == "option")
        {
            "ID_COMMON_SELECT"
        } else {
            "#"
        };
        let title = if menu == EditSkaterMenu::PartThumbnails {
            self.controller
                .category
                .map(|o| o.definition().title)
                .unwrap_or(menu.title())
        } else if menu == EditSkaterMenu::WearStyles
            // GetTitleText 0x82649408: the wear styles of a deck.
            && self.current_part.is_some_and(|part| self.db.parts[part].slot == SKATE_DECK)
        {
            "ID_CUSTOM_EDITDECK"
        } else {
            menu.title()
        };
        Ok(EditSkaterView {
            gender_confirmation: self.gender_confirmation,
            exit_confirmation: self.exit_confirmation,
            visible: self.controller.visible,
            menu,
            title: title.into(),
            rows,
            selected: self.controller.selected(),
            window_start: self.controller.window_start(),
            wallet: self.shop.balance(&self.db),
            help_a: help_a.into(),
            help_b: "ID_COMMON_BACK".into(),
            help_start: if menu == EditSkaterMenu::EditSkaterMain {
                "ID_CAC_FINISHED_TITLE"
            } else {
                "ID_COMMON_CONTINUE"
            }
            .into(),
            cash_popup: self.shop.not_enough_cash,
            receipt: self.receipt_view()?,
        })
    }
    /// `GetPartStatus` (`0x82659be8`); every part is owned under the unlock policy.
    fn part_status(&self, index: usize) -> &'static str {
        if self.policy.unlock_all_clothing { "owned" } else { self.shop.status(&self.db, &self.record, index) }
    }
    fn part_description(&self, index: usize) -> RecordResult<String> {
        let p = &self.db.parts[index];
        Ok(format!(
            "ID_CAC_PART_{}_{}",
            self.db.attribute(p, 38)?,
            self.db.attribute(p, 36)?
        )
        .to_ascii_uppercase())
    }
    fn description_candidates(&self, index: usize) -> RecordResult<Vec<String>> {
        let p = &self.db.parts[index];
        let group = wear_style_group(p);
        let mut out = Vec::new();
        for (i, q) in self.db.parts.iter().enumerate() {
            if q.slot == p.slot
                && q.attributes[13] == p.attributes[13]
                && ((group != u16::MAX && wear_style_group(q) == group) || i == index)
            {
                out.push(self.part_description(i)?);
            }
        }
        out.push(hal(
            "ID_CAC_COMPONENT_TYPE_",
            skate2_formats::cac_parts::SLOT_NAMES[p.slot as usize],
        ));
        Ok(out)
    }
    /// `GetBrandKey` (`0x826496d8`): the first brand filter whose `sponsor` name (layout +0) is
    /// the brand, 0 when none is.
    fn brand_key(&self, brand: &str) -> RecordResult<u64> {
        for &key in &self.brand_filters {
            if self.vault.instance(SPONSOR, key).try_cached_layout_text(0)?.is_some_and(|name| name.bytes() == brand.as_bytes()) {
                return Ok(key);
            }
        }
        Ok(0)
    }
    /// `GetSponsorLogoFileName` (`0x82652f28`) of a part in the part thumbnails: the logo file
    /// (layout +4) of the `sponsor` its brand (`pManufacturerID`, `no_brand` without one) names;
    /// `None` for `no_image` when no sponsor has the brand's name or the sponsor has no logo.
    fn sponsor_logo(&self, index: usize) -> RecordResult<Option<String>> {
        let brand = self.db.attribute(&self.db.parts[index], 34)?;
        let key = self.brand_key(if brand.is_empty() { "no_brand" } else { brand })?;
        let Some(logo) = self.vault.instance(SPONSOR, key).try_cached_layout_text(4)? else { return Ok(None) };
        Ok(Some(std::str::from_utf8(logo.bytes())?.to_owned()))
    }
}
fn flag(v: u8) -> bool {
    v != 0 && v != 255
}
fn hal(prefix: &str, value: &str) -> String {
    // HALUtils::HALifyString82669738: uppercase, then replace each literal
    // space with '_'. Do not strip punctuation or collapse repeated spaces.
    format!("{prefix}{value}").to_ascii_uppercase().replace(' ', "_")
}
fn menu_option(m: EditSkaterMenu) -> Option<EditSkaterOption> {
    use EditSkaterMenu as M;
    use EditSkaterOption as O;
    Some(match m {
        M::Skin => O::Skin,
        M::BodyShape => O::BodyShape,
        M::Hair => O::Hair,
        M::FacialHair => O::FacialHair,
        M::Eyes => O::Eyes,
        M::Brow => O::Brow,
        M::Nose => O::Nose,
        M::Jaw => O::Jaw,
        M::Mouth => O::Mouth,
        M::Chin => O::Chin,
        _ => return None,
    })
}
/// SetScreenState(10), 0x8268518c..0x82685ed0: GetStringHash inputs in
/// eAnimGestureNames order. These names are unlock IDs, not HAL or clip names.
const GESTURE_UNLOCK_NAMES: [&str; 35] = [
    "gesture_sky",
    "gesture_what",
    "gesture_snap",
    "gesture_thumbdown",
    "gesture_pumphi",
    "gesture_fonz",
    "gesture_bruce",
    "gesture_shrug",
    "gesture_pumplo",
    "gesture_devil",
    "gesture_fu",
    "gesture_throwarms",
    "gesture_doublegun",
    "gesture_peace",
    "gesture_shaka",
    "gesture_spock",
    "gesture_airguitar",
    "gesture_airplane",
    "gesture_flex",
    "gesture_boxing",
    "gesture_clothesline",
    "gesture_paper",
    "gesture_rock",
    "gesture_scissors",
    "gesture_dunno",
    "gesture_fingerpoint",
    "gesture_fingerwag",
    "gesture_fists",
    "gesture_fliptable",
    "gesture_freedom",
    "gesture_getouttahere",
    "gesture_push",
    "gesture_raisetheroof",
    "gesture_wings",
    "gesture_axe",
];
const GESTURE_HAL: [&str; 35] = [
    "ID_GESTURE_SKY",
    "ID_GESTURE_WHAT",
    "ID_GESTURE_SNAP",
    "ID_GESTURE_THMB_DWN",
    "ID_GESTURE_PUMP_HI",
    "ID_GESTURE_FONZ",
    "ID_GESTURE_BRUCE",
    "ID_GESTURE_SHRUG",
    "ID_GESTURE_PUMP_LO",
    "ID_GESTURE_DEVIL",
    "ID_GESTURE_FU",
    "ID_GESTURE_THROWARMS",
    "ID_GESTURE_DOUBLE_GUN",
    "ID_GESTURE_PEACE",
    "ID_GESTURE_SHAKA",
    "ID_GESTURE_SPOCK",
    "ID_GESTURE_AIRGUITAR",
    "ID_GESTURE_AIRPLANE",
    "ID_GESTURE_BICEPFLEX",
    "ID_GESTURE_BOXING",
    "ID_GESTURE_CLOTHESLINE",
    "ID_GESTURE_COM_PAPER",
    "ID_GESTURE_COM_ROCK",
    "ID_GESTURE_COM_SCISSORS",
    "ID_GESTURE_DUNNO",
    "ID_GESTURE_FINGERPOINT",
    "ID_GESTURE_FINGERWAG",
    "ID_GESTURE_FISTS",
    "ID_GESTURE_FLIPTABLE",
    "ID_GESTURE_FREEDOM",
    "ID_GESTURE_GETOUTTAHERE",
    "ID_GESTURE_PUSH",
    "ID_GESTURE_RAISEROOF",
    "ID_GESTURE_WINGS",
    "ID_GESTURE_AXE_SPRAY",
];

// Native constructor82680054..82680470 std stores. Full64 words recovered
// from lis/ori/insrdi, since Hex-Rays loses high halves in this constructor.
const GESTURE_BODY: [u64; 35] = [
    0x55a60f8bf2e662e8,
    0xb27deaf46386b288,
    0xeaa19b768d9b222f,
    0x81c7c64d5290e218,
    0xb0742355dc5b5c9b,
    0xd1396fdca3740cf8,
    0x6d66ea0b63a7440e,
    0x04883df1614f6ca0,
    0xa93dd26de9c84ea6,
    0xd5def8f55620fb08,
    0x9a1f454a38f70280,
    0x130ff44033181bb6,
    0xad17a9a2283dd9ca,
    0x362c06c263b593bd,
    0x79027f8466ffa848,
    0x1ef1de28802ea038,
    0xbcacffaa652fac9b,
    0xb4689c41b8329176,
    0x151e783eea2f0cbe,
    0x55c88e3f1634398f,
    0x5f72c4925a3b33a5,
    0x45ef058bc984c7e8,
    0xba902cf0210f14af,
    0x9faad84bf3571754,
    0xb0dc570164161df6,
    0x7138813fcd93e42d,
    0xcf3d7cdc67c109ea,
    0xe0a914a40bf94ef2,
    0x60c67224e6fdcc28,
    0xd85e25f874803d8b,
    0x6c53a0716d15efe2,
    0x06110c5bd81cdf44,
    0xe6a8e1c2514e0095,
    0xcb97ca0e1ddbdf14,
    0xbf20555843faa417,
];
